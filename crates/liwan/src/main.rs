use anyhow::Result;

use liwan::app::{
    Liwan,
    models::{Event, EventExit},
};
use liwan::{cli, config::Config, web};
use tracing_subscriber::EnvFilter;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = cli::args();
    setup_logger(args.log_level)?;

    let config = Config::load(args.config, std::env::vars())?;
    let (events_tx, events_rx) = tokio::sync::mpsc::channel::<Event>(10240);
    let (exits_tx, exits_rx) = tokio::sync::mpsc::channel::<EventExit>(10240);
    let queues = web::EventQueues { events: events_tx, exits: exits_tx };

    if let Some(cmd) = args.cmd {
        return cli::handle_command(config, cmd);
    }

    let app = Liwan::try_new(config)?;
    app.run_background_tasks();

    let interval = std::time::Duration::try_from(
        app.config.duckdb.checkpoint_interval.to_duration(jiff::SpanRelativeTo::days_are_24_hours())?,
    )?;
    let (stop_checkpoints, mut checkpoint_shutdown) = tokio::sync::oneshot::channel::<()>();
    let checkpoints = (!interval.is_zero()).then(|| {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = &mut checkpoint_shutdown => break,
                    _ = tokio::time::sleep(interval) => {}
                }
                let app = app.clone();
                match tokio::task::spawn_blocking(move || -> Result<()> {
                    app.events_conn()?.execute_batch("CHECKPOINT")?;
                    Ok(())
                })
                .await
                {
                    Ok(Ok(())) => tracing::debug!("Completed periodic DuckDB checkpoint"),
                    Ok(Err(err)) => tracing::warn!(%err, "Periodic DuckDB checkpoint failed"),
                    Err(err) => tracing::error!(%err, "Periodic DuckDB checkpoint task failed"),
                }
            }
        })
    });

    let server = web::start_webserver(app.clone(), queues);
    let event_processor = app.events.process_events(events_rx);
    let exit_processor = app.events.process_exits(exits_rx);
    tokio::pin!(server, event_processor, exit_processor);

    tokio::select! {
        biased;
        res = &mut server => res?,
        res = &mut event_processor => return res,
        res = &mut exit_processor => return res,
    };

    // The stopped server has dropped its senders. Drain every event it accepted before checkpointing.
    drop(stop_checkpoints);
    event_processor.await?;
    exit_processor.await?;
    if let Some(checkpoints) = checkpoints {
        checkpoints.await?;
    }
    app.shutdown()
}

fn setup_logger(log_level: tracing::Level) -> Result<()> {
    // external crates should use WARN
    let mut filter = EnvFilter::from_default_env()
        .add_directive(format!("{}={}", env!("CARGO_PKG_NAME"), log_level).parse()?)
        .add_directive(tracing::Level::WARN.into());
    if log_level == tracing::Level::DEBUG || log_level == tracing::Level::TRACE {
        filter = filter.add_directive(format!("tower_http::trace={log_level}").parse()?);
    }

    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    #[cfg(debug_assertions)]
    tracing::info!("Running in debug mode");
    Ok(())
}

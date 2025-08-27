use eyre::Result;
use tokio::task::JoinSet;

pub async fn shutdown() -> Result<()> {
    let mut shutdown = JoinSet::new();
    shutdown.spawn(tokio::signal::ctrl_c());

    #[cfg(unix)]
    shutdown.spawn(async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        sigterm.recv().await;
        Ok(())
    });

    shutdown.join_next().await.expect("there are tasks")??;
    Ok(())
}

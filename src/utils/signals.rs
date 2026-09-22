/// Waits for the process shutdown signal.
pub async fn shutdown() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            tracing::error!(?error, "Failed to listen for Ctrl-C");
                            sigterm.recv().await;
                        }
                    }
                    _ = sigterm.recv() => {}
                }
            }
            Err(error) => {
                tracing::error!(?error, "Failed to listen for SIGTERM");
                if let Err(error) = tokio::signal::ctrl_c().await {
                    tracing::error!(?error, "Failed to listen for Ctrl-C");
                }
            }
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(?error, "Failed to listen for Ctrl-C");
    }
}

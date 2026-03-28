use anyhow::Result;

use shellwright::config::Config;
use shellwright::daemon::server::DaemonServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("shellwright=info".parse().unwrap()),
        )
        .with_target(false)
        .json()
        .init();

    let config = Config::default();

    tracing::info!(
        data_dir = %config.data_dir.display(),
        max_sessions = config.max_sessions,
        "Shellwright daemon starting"
    );

    let server = DaemonServer::new(config);
    server.run().await
}

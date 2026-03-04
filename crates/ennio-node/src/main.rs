mod auth;
mod server;
mod service;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "ennio-node",
    version,
    about = "Remote node daemon for Ennio agent orchestration"
)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "9100")]
    port: u16,

    /// Idle timeout in seconds before self-termination
    #[arg(long, default_value = "3600")]
    idle_timeout: u64,

    /// Root directory for workspaces
    #[arg(long)]
    workspace_root: Option<String>,

    /// Bearer token for gRPC authentication
    #[arg(long, env = "ENNIO_NODE_AUTH_TOKEN")]
    auth_token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    let addr = format!("127.0.0.1:{}", args.port);

    tracing::info!(
        addr = %addr,
        idle_timeout_secs = args.idle_timeout,
        auth = args.auth_token.is_some(),
        "starting ennio-node daemon"
    );

    server::run(
        &addr,
        args.idle_timeout,
        args.workspace_root.as_deref(),
        args.auth_token.as_deref(),
    )
    .await?;

    Ok(())
}

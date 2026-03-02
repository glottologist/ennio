use std::sync::Arc;

use ennio_proto::EnnioNodeServer;
use tonic::transport::Server;
use tracing::info;

use crate::service::EnnioNodeService;

pub async fn run(
    addr: &str,
    idle_timeout_secs: u64,
    workspace_root: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = EnnioNodeService::new(idle_timeout_secs, workspace_root);
    let service = Arc::new(service);

    let idle_service = Arc::clone(&service);
    let idle_timeout = std::time::Duration::from_secs(idle_timeout_secs);

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            if idle_service.idle_exceeded(idle_timeout) {
                info!("idle timeout exceeded, shutting down");
                std::process::exit(0);
            }
        }
    });

    let parsed_addr: std::net::SocketAddr = addr.parse()?;

    info!(addr = %parsed_addr, "gRPC server listening");

    Server::builder()
        .add_service(EnnioNodeServer::from_arc(service))
        .serve(parsed_addr)
        .await?;

    Ok(())
}

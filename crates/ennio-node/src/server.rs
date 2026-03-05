use std::sync::Arc;

use ennio_proto::EnnioNodeServer;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;
use tracing::{info, warn};

use crate::auth;
use crate::service::EnnioNodeService;

pub async fn run(
    addr: &str,
    idle_timeout_secs: u64,
    workspace_root: Option<&str>,
    auth_token: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let shutdown_token = CancellationToken::new();

    let service = EnnioNodeService::new(idle_timeout_secs, workspace_root, shutdown_token.clone()); // clone: CancellationToken is Arc-based
    let service = Arc::new(service);

    let idle_service = Arc::clone(&service);
    let idle_timeout = std::time::Duration::from_secs(idle_timeout_secs);
    let idle_token = shutdown_token.clone(); // clone: CancellationToken is Arc-based

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            if idle_service.idle_exceeded(idle_timeout) {
                info!("idle timeout exceeded, shutting down");
                idle_token.cancel();
                return;
            }
        }
    });

    let parsed_addr: std::net::SocketAddr = addr.parse()?;

    match auth_token {
        Some(token) => {
            info!(addr = %parsed_addr, "gRPC server listening (auth enabled)");
            let interceptor = auth::make_interceptor(token.to_owned());
            let grpc_service = EnnioNodeServer::from_arc(service);
            Server::builder()
                .add_service(tonic::service::interceptor::InterceptedService::new(
                    grpc_service,
                    interceptor,
                ))
                .serve_with_shutdown(parsed_addr, shutdown_token.cancelled())
                .await?;
        }
        None => {
            warn!(addr = %parsed_addr, "gRPC server listening WITHOUT authentication — all requests will be accepted");
            Server::builder()
                .add_service(EnnioNodeServer::from_arc(service))
                .serve_with_shutdown(parsed_addr, shutdown_token.cancelled())
                .await?;
        }
    }

    Ok(())
}

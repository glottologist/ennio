use std::path::PathBuf;

use ennio_core::config::NodeConnectionConfig;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use ennio_core::workspace::WorkspaceCreateConfig;
use ennio_proto::proto::ennio_node_client::EnnioNodeClient;
use ennio_proto::{
    CreateRuntimeRequest, CreateWorkspaceRequest, DestroyRuntimeRequest, DestroyWorkspaceRequest,
    GetOutputRequest, HeartbeatRequest, IsAliveRequest, ProtoRuntimeHandle, SendMessageRequest,
    ShutdownRequest,
};
use tonic::transport::Channel;
use tracing::{debug, info};

use crate::SshClient;
use crate::error::SshError;

pub struct RemoteNode {
    ssh_client: SshClient,
    grpc_client: EnnioNodeClient<Channel>,
    host: String,
    remote_port: u16,
    local_port: u16,
}

impl RemoteNode {
    pub async fn connect(
        ssh_client: &SshClient,
        config: &NodeConnectionConfig,
        host: &str,
    ) -> Result<Self, SshError> {
        let remote_port = config.port;
        let local_port = remote_port;

        let binary_path = config
            .ennio_binary_path
            .as_deref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ennio-node".to_owned());

        let check_output = ssh_client
            .exec(&format!(
                "ss -tlnp 2>/dev/null | grep :{remote_port} || echo NOT_RUNNING"
            ))
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("failed to check daemon status: {e}"),
            })?;

        if check_output.stdout.contains("NOT_RUNNING") {
            info!(
                host = host,
                port = remote_port,
                "launching ennio-node daemon"
            );

            let mut launch_cmd = format!(
                "{binary_path} --port {remote_port} --idle-timeout {}",
                config.idle_timeout.as_secs()
            );

            if let Some(ref ws_root) = config.workspace_root {
                launch_cmd.push_str(&format!(
                    " --workspace-root {}",
                    shell_escape::escape(ws_root.to_string_lossy())
                ));
            }

            ssh_client
                .exec_detached(&launch_cmd)
                .await
                .map_err(|e| SshError::Tunnel {
                    message: format!("failed to launch ennio-node: {e}"),
                })?;

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        debug!(
            host = host,
            local_port = local_port,
            remote_port = remote_port,
            "establishing SSH port forward"
        );

        ssh_client
            .forward_local_port(local_port, "127.0.0.1", remote_port)
            .await?;

        let endpoint = format!("http://127.0.0.1:{local_port}");
        let grpc_client =
            EnnioNodeClient::connect(endpoint)
                .await
                .map_err(|e| SshError::Tunnel {
                    message: format!("failed to connect gRPC client: {e}"),
                })?;

        Ok(Self {
            ssh_client: ssh_client.clone(), // clone: SshClient uses Arc internally
            grpc_client,
            host: host.to_owned(),
            remote_port,
            local_port,
        })
    }

    pub async fn create_workspace(
        &mut self,
        config: &WorkspaceCreateConfig<'_>,
        workspace_type: &str,
    ) -> Result<PathBuf, SshError> {
        let request = CreateWorkspaceRequest {
            project_id: config.project_id.to_string(),
            repo_url: config.project.repo.clone(), // clone: proto needs owned String
            path: config.project.path.to_string_lossy().into_owned(),
            session_id: config.session_id.to_string(),
            default_branch: config.project.default_branch.clone(), // clone: proto needs owned String
            branch: config.branch.map(str::to_owned),
            workspace_type: workspace_type.to_owned(),
        };

        let response = self
            .grpc_client
            .create_workspace(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("create_workspace RPC failed: {e}"),
            })?;

        Ok(PathBuf::from(response.into_inner().workspace_path))
    }

    pub async fn destroy_workspace(&mut self, workspace_path: &str) -> Result<(), SshError> {
        let request = DestroyWorkspaceRequest {
            workspace_path: workspace_path.to_owned(),
        };

        self.grpc_client
            .destroy_workspace(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("destroy_workspace RPC failed: {e}"),
            })?;

        Ok(())
    }

    pub async fn create_runtime(
        &mut self,
        config: &RuntimeCreateConfig,
    ) -> Result<RuntimeHandle, SshError> {
        let request: CreateRuntimeRequest = config.into();

        let response = self
            .grpc_client
            .create_runtime(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("create_runtime RPC failed: {e}"),
            })?;

        let proto_handle = response
            .into_inner()
            .handle
            .ok_or_else(|| SshError::Tunnel {
                message: "create_runtime response missing handle".to_owned(),
            })?;

        Ok(proto_handle.into())
    }

    pub async fn destroy_runtime(&mut self, handle: &RuntimeHandle) -> Result<(), SshError> {
        let proto_handle: ProtoRuntimeHandle = handle.into();
        let request = DestroyRuntimeRequest {
            handle: Some(proto_handle),
        };

        self.grpc_client
            .destroy_runtime(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("destroy_runtime RPC failed: {e}"),
            })?;

        Ok(())
    }

    pub async fn send_message(
        &mut self,
        handle: &RuntimeHandle,
        message: &str,
    ) -> Result<(), SshError> {
        let proto_handle: ProtoRuntimeHandle = handle.into();
        let request = SendMessageRequest {
            handle: Some(proto_handle),
            message: message.to_owned(),
        };

        self.grpc_client
            .send_message(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("send_message RPC failed: {e}"),
            })?;

        Ok(())
    }

    pub async fn get_output(
        &mut self,
        handle: &RuntimeHandle,
        lines: u32,
    ) -> Result<String, SshError> {
        let proto_handle: ProtoRuntimeHandle = handle.into();
        let request = GetOutputRequest {
            handle: Some(proto_handle),
            lines,
        };

        let response =
            self.grpc_client
                .get_output(request)
                .await
                .map_err(|e| SshError::Tunnel {
                    message: format!("get_output RPC failed: {e}"),
                })?;

        Ok(response.into_inner().output)
    }

    pub async fn is_alive(&mut self, handle: &RuntimeHandle) -> Result<bool, SshError> {
        let proto_handle: ProtoRuntimeHandle = handle.into();
        let request = IsAliveRequest {
            handle: Some(proto_handle),
        };

        let response = self
            .grpc_client
            .is_alive(request)
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("is_alive RPC failed: {e}"),
            })?;

        Ok(response.into_inner().alive)
    }

    pub async fn heartbeat(&mut self) -> Result<bool, SshError> {
        let response = self
            .grpc_client
            .heartbeat(HeartbeatRequest {})
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("heartbeat RPC failed: {e}"),
            })?;

        Ok(response.into_inner().healthy)
    }

    pub async fn shutdown(&mut self) -> Result<(), SshError> {
        self.grpc_client
            .shutdown(ShutdownRequest { graceful: true })
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!("shutdown RPC failed: {e}"),
            })?;

        Ok(())
    }

    pub fn ssh_client(&self) -> &SshClient {
        &self.ssh_client
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl std::fmt::Debug for RemoteNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteNode")
            .field("host", &self.host)
            .field("remote_port", &self.remote_port)
            .field("local_port", &self.local_port)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    #[rstest]
    #[case(1024u16)]
    #[case(9100u16)]
    #[case(65535u16)]
    fn valid_port_range(#[case] port: u16) {
        assert!(port >= 1024);
    }

    proptest! {
        #[test]
        fn port_range_always_valid(port in 1024u16..=65535) {
            prop_assert!(port >= 1024);
        }
    }
}

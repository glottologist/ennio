use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ennio_core::config::NodeConnectionConfig;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use ennio_core::workspace::WorkspaceCreateConfig;
use ennio_proto::proto::ennio_node_client::EnnioNodeClient;
use ennio_proto::{
    CreateRuntimeRequest, CreateWorkspaceRequest, DestroyRuntimeRequest, DestroyWorkspaceRequest,
    GetOutputRequest, HeartbeatRequest, IsAliveRequest, ProtoRuntimeHandle, SendMessageRequest,
    ShutdownRequest,
};
use secrecy::ExposeSecret;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;
use tracing::{debug, info};

use crate::SshClient;
use crate::error::SshError;

const NODE_READY_POLL_INTERVAL: Duration = Duration::from_millis(250);
const NODE_READY_MAX_ATTEMPTS: u32 = 20;
const TOKEN_BYTES: usize = 32;

#[derive(Clone)]
struct AuthInterceptor {
    token: Option<Arc<str>>,
}

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(ref token) = self.token {
            let header_value: tonic::metadata::MetadataValue<tonic::metadata::Ascii> =
                format!("Bearer {token}")
                    .parse()
                    .map_err(|_| tonic::Status::internal("invalid token format"))?;
            req.metadata_mut().insert("authorization", header_value);
        }
        Ok(req)
    }
}

fn generate_node_token() -> Result<String, SshError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| SshError::Execution {
        command: String::new(),
        message: format!("failed to generate auth token: {e}"),
    })?;
    Ok(hex::encode(bytes))
}

pub struct RemoteNode {
    ssh_client: SshClient,
    grpc_client: EnnioNodeClient<InterceptedService<Channel, AuthInterceptor>>,
    host: String,
    remote_port: u16,
    local_port: u16,
}

fn resolve_auth_token(config: &NodeConnectionConfig) -> Result<Option<String>, SshError> {
    match &config.auth_token {
        Some(token) => Ok(Some(token.expose_secret().to_owned())),
        None => Ok(Some(generate_node_token()?)),
    }
}

fn resolve_binary_path(config: &NodeConnectionConfig) -> String {
    config
        .ennio_binary_path
        .as_deref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ennio-node".to_owned())
}

fn build_launch_command(
    config: &NodeConnectionConfig,
    binary_path: &str,
    auth_token: Option<&str>,
) -> String {
    let escaped_binary = shell_escape::escape(binary_path.into());
    let mut cmd = format!(
        "{escaped_binary} --port {} --idle-timeout {}",
        config.port,
        config.idle_timeout.as_secs()
    );

    if let Some(ref ws_root) = config.workspace_root {
        cmd.push_str(&format!(
            " --workspace-root {}",
            shell_escape::escape(ws_root.to_string_lossy())
        ));
    }

    match auth_token {
        Some(token) => {
            let escaped_token = shell_escape::escape(token.into());
            format!("bash -c 'export ENNIO_NODE_AUTH_TOKEN={escaped_token}; exec {cmd}'")
        }
        None => cmd,
    }
}

async fn ensure_daemon_running(
    ssh_client: &SshClient,
    config: &NodeConnectionConfig,
    host: &str,
    auth_token: Option<&str>,
    binary_path: &str,
) -> Result<(), SshError> {
    let remote_port = config.port;
    let port_check_cmd = format!("ss -tlnp 2>/dev/null | grep :{remote_port} || echo NOT_RUNNING");

    let check_output = ssh_client
        .exec(&port_check_cmd)
        .await
        .map_err(|e| SshError::Tunnel {
            message: format!("failed to check daemon status: {e}"),
        })?;

    if !check_output.stdout.contains("NOT_RUNNING") {
        return Ok(());
    }

    info!(
        host = host,
        port = remote_port,
        "launching ennio-node daemon"
    );

    let launch_cmd = build_launch_command(config, binary_path, auth_token);
    ssh_client
        .exec_detached(&launch_cmd)
        .await
        .map_err(|e| SshError::Tunnel {
            message: format!("failed to launch ennio-node: {e}"),
        })?;

    poll_daemon_ready(ssh_client, &port_check_cmd).await
}

async fn poll_daemon_ready(ssh_client: &SshClient, port_check_cmd: &str) -> Result<(), SshError> {
    for attempt in 0..NODE_READY_MAX_ATTEMPTS {
        if let Ok(output) = ssh_client.exec(port_check_cmd).await {
            if !output.stdout.contains("NOT_RUNNING") {
                debug!(attempt, "ennio-node daemon is ready");
                return Ok(());
            }
        }
        tokio::time::sleep(NODE_READY_POLL_INTERVAL).await;
    }
    Err(SshError::Timeout {
        duration: NODE_READY_POLL_INTERVAL * NODE_READY_MAX_ATTEMPTS,
    })
}

async fn connect_grpc_client(
    local_port: u16,
    auth_token: Option<String>,
) -> Result<EnnioNodeClient<InterceptedService<Channel, AuthInterceptor>>, SshError> {
    let endpoint = format!("http://127.0.0.1:{local_port}");
    let channel = Channel::from_shared(endpoint)
        .map_err(|e| SshError::Tunnel {
            message: format!("invalid endpoint: {e}"),
        })?
        .connect()
        .await
        .map_err(|e| SshError::Tunnel {
            message: format!("failed to connect gRPC client: {e}"),
        })?;

    let interceptor = AuthInterceptor {
        token: auth_token.map(|t| Arc::from(t.as_str())),
    };
    Ok(EnnioNodeClient::with_interceptor(channel, interceptor))
}

impl RemoteNode {
    pub async fn connect(
        ssh_client: &SshClient,
        config: &NodeConnectionConfig,
        host: &str,
    ) -> Result<Self, SshError> {
        let remote_port = config.port;
        let local_port = remote_port;
        let auth_token = resolve_auth_token(config)?;
        let binary_path = resolve_binary_path(config);

        ensure_daemon_running(
            ssh_client,
            config,
            host,
            auth_token.as_deref(),
            &binary_path,
        )
        .await?;

        debug!(
            host = host,
            local_port = local_port,
            remote_port = remote_port,
            "establishing SSH port forward"
        );

        ssh_client
            .forward_local_port(local_port, "127.0.0.1", remote_port)
            .await?;

        let grpc_client = connect_grpc_client(local_port, auth_token).await?;

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

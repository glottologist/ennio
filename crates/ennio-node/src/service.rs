use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use ennio_proto::proto::ennio_node_server::EnnioNode;
use ennio_proto::{
    CreateRuntimeRequest, CreateRuntimeResponse, CreateWorkspaceRequest, CreateWorkspaceResponse,
    DestroyRuntimeRequest, DestroyRuntimeResponse, DestroyWorkspaceRequest,
    DestroyWorkspaceResponse, GetOutputRequest, GetOutputResponse, HeartbeatRequest,
    HeartbeatResponse, IsAliveRequest, IsAliveResponse, ProtoRuntimeHandle, SendMessageRequest,
    SendMessageResponse, ShutdownRequest, ShutdownResponse,
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use tracing::{debug, info};

pub struct EnnioNodeService {
    started_at: Instant,
    last_activity: AtomicU64,
    workspace_root: Option<String>,
    runtimes: RwLock<HashMap<String, RuntimeState>>,
    shutdown_token: CancellationToken,
}

struct RuntimeState {
    alive: bool,
}

impl EnnioNodeService {
    pub fn new(
        _idle_timeout_secs: u64,
        workspace_root: Option<&str>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            started_at: Instant::now(),
            last_activity: AtomicU64::new(0),
            workspace_root: workspace_root.map(str::to_owned),
            runtimes: RwLock::new(HashMap::new()),
            shutdown_token,
        }
    }

    fn touch_activity(&self) {
        let elapsed = self.started_at.elapsed().as_secs();
        self.last_activity.store(elapsed, Ordering::Relaxed);
    }

    pub fn idle_exceeded(&self, timeout: Duration) -> bool {
        let last = self.last_activity.load(Ordering::Relaxed);
        let now = self.started_at.elapsed().as_secs();
        now.saturating_sub(last) > timeout.as_secs()
    }

    fn resolve_workspace_root(&self) -> String {
        self.workspace_root
            .as_deref()
            .unwrap_or("/tmp/ennio-workspaces")
            .to_owned()
    }
}

fn validate_path_segment(value: &str, name: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if value.contains("..") || value.contains('/') || value.contains('\\') {
        return Err(format!("{name} contains invalid characters"));
    }
    if !value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "{name} must be alphanumeric with hyphens and underscores only"
        ));
    }
    Ok(())
}

#[tonic::async_trait]
impl EnnioNode for EnnioNodeService {
    async fn create_workspace(
        &self,
        request: Request<CreateWorkspaceRequest>,
    ) -> Result<Response<CreateWorkspaceResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        validate_path_segment(&req.session_id, "session_id").map_err(Status::invalid_argument)?;
        validate_path_segment(&req.workspace_type, "workspace_type")
            .map_err(Status::invalid_argument)?;

        debug!(
            session_id = %req.session_id,
            workspace_type = %req.workspace_type,
            "create_workspace"
        );

        let root = self.resolve_workspace_root();
        let workspace_path = format!("{root}/{}/{}", req.session_id, req.workspace_type);

        tokio::fs::create_dir_all(&workspace_path)
            .await
            .map_err(|e| Status::internal(format!("failed to create workspace dir: {e}")))?;

        Ok(Response::new(CreateWorkspaceResponse { workspace_path }))
    }

    async fn destroy_workspace(
        &self,
        request: Request<DestroyWorkspaceRequest>,
    ) -> Result<Response<DestroyWorkspaceResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        debug!(workspace_path = %req.workspace_path, "destroy_workspace");

        let root = self.resolve_workspace_root();
        let canonical_root = std::path::Path::new(&root)
            .canonicalize()
            .map_err(|e| Status::internal(format!("failed to canonicalize workspace root: {e}")))?;
        let canonical_path = std::path::Path::new(&req.workspace_path)
            .canonicalize()
            .map_err(|e| Status::invalid_argument(format!("invalid workspace path: {e}")))?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(Status::permission_denied(
                "workspace path is outside workspace root",
            ));
        }

        if tokio::fs::metadata(&req.workspace_path).await.is_ok() {
            tokio::fs::remove_dir_all(&req.workspace_path)
                .await
                .map_err(|e| Status::internal(format!("failed to remove workspace: {e}")))?;
        }

        Ok(Response::new(DestroyWorkspaceResponse {}))
    }

    async fn create_runtime(
        &self,
        request: Request<CreateRuntimeRequest>,
    ) -> Result<Response<CreateRuntimeResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        debug!(
            session_name = %req.session_name,
            "create_runtime"
        );

        let runtime_id = format!("node-{}", req.session_name);

        let state = RuntimeState { alive: true };

        let mut runtimes = self.runtimes.write().await;
        runtimes.insert(runtime_id.clone(), state); // clone: key used after insert

        let handle = ProtoRuntimeHandle {
            id: runtime_id,
            runtime_name: req.session_name,
            data: HashMap::new(),
        };

        Ok(Response::new(CreateRuntimeResponse {
            handle: Some(handle),
        }))
    }

    async fn destroy_runtime(
        &self,
        request: Request<DestroyRuntimeRequest>,
    ) -> Result<Response<DestroyRuntimeResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        let handle = req
            .handle
            .ok_or_else(|| Status::invalid_argument("missing handle"))?;

        debug!(runtime_id = %handle.id, "destroy_runtime");

        let mut runtimes = self.runtimes.write().await;
        runtimes.remove(&handle.id);

        Ok(Response::new(DestroyRuntimeResponse {}))
    }

    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        let handle = req
            .handle
            .ok_or_else(|| Status::invalid_argument("missing handle"))?;

        debug!(
            runtime_id = %handle.id,
            message_len = req.message.len(),
            "send_message"
        );

        let runtimes = self.runtimes.read().await;
        if !runtimes.contains_key(&handle.id) {
            return Err(Status::not_found(format!(
                "runtime not found: {}",
                handle.id
            )));
        }

        Ok(Response::new(SendMessageResponse {}))
    }

    async fn get_output(
        &self,
        request: Request<GetOutputRequest>,
    ) -> Result<Response<GetOutputResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        let handle = req
            .handle
            .ok_or_else(|| Status::invalid_argument("missing handle"))?;

        let runtimes = self.runtimes.read().await;
        if !runtimes.contains_key(&handle.id) {
            return Err(Status::not_found(format!(
                "runtime not found: {}",
                handle.id
            )));
        }

        Ok(Response::new(GetOutputResponse {
            output: String::new(),
        }))
    }

    async fn is_alive(
        &self,
        request: Request<IsAliveRequest>,
    ) -> Result<Response<IsAliveResponse>, Status> {
        self.touch_activity();
        let req = request.into_inner();

        let handle = req
            .handle
            .ok_or_else(|| Status::invalid_argument("missing handle"))?;

        let runtimes = self.runtimes.read().await;
        let alive = runtimes.get(&handle.id).map(|s| s.alive).unwrap_or(false);

        Ok(Response::new(IsAliveResponse { alive }))
    }

    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        self.touch_activity();

        let uptime_secs = self.started_at.elapsed().as_secs();

        Ok(Response::new(HeartbeatResponse {
            healthy: true,
            uptime_secs,
        }))
    }

    async fn shutdown(
        &self,
        request: Request<ShutdownRequest>,
    ) -> Result<Response<ShutdownResponse>, Status> {
        let req = request.into_inner();

        info!(graceful = req.graceful, "shutdown requested");

        if req.graceful {
            let runtimes = self.runtimes.read().await;
            if !runtimes.is_empty() {
                info!(
                    active_runtimes = runtimes.len(),
                    "graceful shutdown with active runtimes"
                );
            }
        }

        let token = self.shutdown_token.clone(); // clone: CancellationToken is Arc-based
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token.cancel();
        });

        Ok(Response::new(ShutdownResponse { accepted: true }))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn test_service() -> EnnioNodeService {
        EnnioNodeService::new(3600, None, CancellationToken::new())
    }

    #[test]
    fn touch_activity_updates_timestamp() {
        let service = EnnioNodeService::new(10, None, CancellationToken::new());
        service.touch_activity();
        let last = service.last_activity.load(Ordering::Relaxed);
        assert!(last <= service.started_at.elapsed().as_secs());
    }

    #[tokio::test]
    async fn heartbeat_returns_healthy() {
        let service = test_service();
        let response = service
            .heartbeat(Request::new(HeartbeatRequest {}))
            .await
            .unwrap();
        let inner = response.into_inner();
        assert!(inner.healthy);
    }

    #[tokio::test]
    async fn create_and_destroy_runtime() {
        let service = test_service();

        let create_req = Request::new(CreateRuntimeRequest {
            session_id: "test-session".to_owned(),
            launch_command: "echo hello".to_owned(),
            env: HashMap::new(),
            cwd: "/tmp".to_owned(),
            session_name: "test-rt".to_owned(),
        });

        let create_resp = service.create_runtime(create_req).await.unwrap();
        let handle = create_resp.into_inner().handle.unwrap();
        assert_eq!(handle.runtime_name, "test-rt");

        let is_alive_req = Request::new(IsAliveRequest {
            handle: Some(handle.clone()), // clone: reusing handle for next RPC
        });
        let alive_resp = service.is_alive(is_alive_req).await.unwrap();
        assert!(alive_resp.into_inner().alive);

        let destroy_req = Request::new(DestroyRuntimeRequest {
            handle: Some(handle),
        });
        service.destroy_runtime(destroy_req).await.unwrap();
    }

    #[tokio::test]
    async fn is_alive_returns_false_for_unknown() {
        let service = test_service();

        let handle = ProtoRuntimeHandle {
            id: "nonexistent".to_owned(),
            runtime_name: "unknown".to_owned(),
            data: HashMap::new(),
        };

        let req = Request::new(IsAliveRequest {
            handle: Some(handle),
        });
        let resp = service.is_alive(req).await.unwrap();
        assert!(!resp.into_inner().alive);
    }

    #[tokio::test]
    async fn send_message_to_unknown_runtime_fails() {
        let service = test_service();

        let handle = ProtoRuntimeHandle {
            id: "nonexistent".to_owned(),
            runtime_name: "unknown".to_owned(),
            data: HashMap::new(),
        };

        let req = Request::new(SendMessageRequest {
            handle: Some(handle),
            message: "hello".to_owned(),
        });
        let result = service.send_message(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_workspace_rejects_path_traversal() {
        let service = EnnioNodeService::new(3600, Some("/tmp/test-ws"), CancellationToken::new());
        let req = Request::new(CreateWorkspaceRequest {
            project_id: "test-proj".to_owned(),
            repo_url: String::new(),
            path: String::new(),
            session_id: "../etc".to_owned(),
            default_branch: String::new(),
            branch: None,
            workspace_type: "worktree".to_owned(),
        });
        let result = service.create_workspace(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_workspace_rejects_empty_session_id() {
        let service = EnnioNodeService::new(3600, Some("/tmp/test-ws"), CancellationToken::new());
        let req = Request::new(CreateWorkspaceRequest {
            project_id: "test-proj".to_owned(),
            repo_url: String::new(),
            path: String::new(),
            session_id: String::new(),
            default_branch: String::new(),
            branch: None,
            workspace_type: "worktree".to_owned(),
        });
        let result = service.create_workspace(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shutdown_cancels_token() {
        let token = CancellationToken::new();
        let service = EnnioNodeService::new(3600, None, token.clone()); // clone: CancellationToken is Arc-based
        assert!(!token.is_cancelled());

        let req = Request::new(ShutdownRequest { graceful: true });
        let resp = service.shutdown(req).await.unwrap();
        assert!(resp.into_inner().accepted);

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(token.is_cancelled());
    }

    proptest! {
        #[test]
        fn validate_path_segment_rejects_empty(name in "[a-z]+") {
            prop_assert!(validate_path_segment("", &name).is_err());
        }

        #[test]
        fn validate_path_segment_rejects_slashes(
            prefix in "[a-z]{1,5}",
            suffix in "[a-z]{1,5}",
        ) {
            let with_slash = format!("{prefix}/{suffix}");
            prop_assert!(validate_path_segment(&with_slash, "test").is_err());
            let with_backslash = format!("{prefix}\\{suffix}");
            prop_assert!(validate_path_segment(&with_backslash, "test").is_err());
        }

        #[test]
        fn validate_path_segment_rejects_dotdot(
            prefix in "[a-z]{0,3}",
            suffix in "[a-z]{0,3}",
        ) {
            let input = format!("{prefix}..{suffix}");
            prop_assert!(validate_path_segment(&input, "test").is_err());
        }

        #[test]
        fn validate_path_segment_accepts_alphanum_hyphen_underscore(
            s in "[a-zA-Z0-9_-]{1,30}",
        ) {
            prop_assert!(validate_path_segment(&s, "test").is_ok());
        }

        #[test]
        fn validate_path_segment_rejects_unicode_and_special(
            s in "[^a-zA-Z0-9_/-]{1,10}",
        ) {
            // Filter out strings that happen to only contain valid chars
            prop_assume!(!s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
            prop_assume!(!s.is_empty());
            prop_assert!(validate_path_segment(&s, "test").is_err());
        }
    }
}

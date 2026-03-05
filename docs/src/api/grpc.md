# gRPC Node Protocol

The `ennio-node` daemon runs on remote machines and communicates with the orchestrator via gRPC over an SSH tunnel.

## Running the Daemon

```bash
ennio-node [OPTIONS]
```

| Option | Default | Env Var | Description |
|--------|---------|---------|-------------|
| `--port` | 9100 | — | gRPC listen port |
| `--idle-timeout` | 3600 | — | Seconds before auto-shutdown |
| `--workspace-root` | — | — | Root directory for workspaces |
| `--auth-token` | — | `ENNIO_NODE_AUTH_TOKEN` | Bearer token for authentication |

## Authentication

When `--auth-token` is set, all gRPC calls must include a `authorization` metadata key with value `Bearer <token>`. The token is compared using constant-time SHA-256 hashing.

If no token is set, the daemon relies on SSH tunnel isolation for security (only reachable via the tunnel).

## Connection Flow

```
Orchestrator                     Remote Host
    │                                │
    ├── SSH connect ────────────────►│
    ├── Port forward (tunnel) ──────►│ :9100
    ├── gRPC Heartbeat ────────────►│
    │◄──────────── healthy=true ────┤
    ├── gRPC CreateWorkspace ──────►│
    │◄──────────── workspace_path ──┤
    ├── gRPC CreateRuntime ────────►│
    │◄──────────── runtime handle ──┤
    │         ...polling...          │
    ├── gRPC Shutdown ─────────────►│
    │◄──────────── accepted=true ───┤
    │                                │
```

## Idle Timeout

The daemon automatically shuts down after `--idle-timeout` seconds of no gRPC activity. This prevents orphaned daemons from consuming resources on remote machines. The orchestrator re-deploys the daemon on next connection.

## Service Definition

```protobuf
service EnnioNode {
  rpc CreateWorkspace(CreateWorkspaceRequest) returns (CreateWorkspaceResponse);
  rpc DestroyWorkspace(DestroyWorkspaceRequest) returns (DestroyWorkspaceResponse);
  rpc CreateRuntime(CreateRuntimeRequest) returns (CreateRuntimeResponse);
  rpc DestroyRuntime(DestroyRuntimeRequest) returns (DestroyRuntimeResponse);
  rpc SendMessage(SendMessageRequest) returns (SendMessageResponse);
  rpc GetOutput(GetOutputRequest) returns (GetOutputResponse);
  rpc IsAlive(IsAliveRequest) returns (IsAliveResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc Shutdown(ShutdownRequest) returns (ShutdownResponse);
}
```

## Message Types

### Workspace Management

```protobuf
message CreateWorkspaceRequest {
  string project_id = 1;
  string repo_url = 2;
  string path = 3;
  string session_id = 4;
  string default_branch = 5;
  optional string branch = 6;
  string workspace_type = 7;    // "worktree" or "clone"
}

message CreateWorkspaceResponse {
  string workspace_path = 1;
}

message DestroyWorkspaceRequest {
  string workspace_path = 1;
}

message DestroyWorkspaceResponse {}
```

### Runtime Management

```protobuf
message CreateRuntimeRequest {
  string session_id = 1;
  string launch_command = 2;
  map<string, string> env = 3;
  string cwd = 4;
  string session_name = 5;
}

message CreateRuntimeResponse {
  ProtoRuntimeHandle handle = 1;
}

message DestroyRuntimeRequest {
  ProtoRuntimeHandle handle = 1;
}

message DestroyRuntimeResponse {}

message ProtoRuntimeHandle {
  string id = 1;
  string runtime_name = 2;
  map<string, string> data = 3;
}
```

### Session Communication

```protobuf
message SendMessageRequest {
  ProtoRuntimeHandle handle = 1;
  string message = 2;
}

message SendMessageResponse {}

message GetOutputRequest {
  ProtoRuntimeHandle handle = 1;
  uint32 lines = 2;
}

message GetOutputResponse {
  string output = 1;
}

message IsAliveRequest {
  ProtoRuntimeHandle handle = 1;
}

message IsAliveResponse {
  bool alive = 1;
}
```

### Health and Lifecycle

```protobuf
message HeartbeatRequest {}

message HeartbeatResponse {
  bool healthy = 1;
  uint64 uptime_secs = 2;
}

message ShutdownRequest {
  bool graceful = 1;
}

message ShutdownResponse {
  bool accepted = 1;
}
```

## RPC Reference

| RPC | Purpose | Request | Response |
|-----|---------|---------|----------|
| `CreateWorkspace` | Create a git worktree or clone on the remote host | project, repo, branch, type | workspace path |
| `DestroyWorkspace` | Remove a workspace directory | workspace path | — |
| `CreateRuntime` | Launch an agent in a tmux session | command, env, cwd, name | runtime handle |
| `DestroyRuntime` | Kill a running agent session | runtime handle | — |
| `SendMessage` | Send text to a running agent | handle, message | — |
| `GetOutput` | Read recent terminal output | handle, line count | output text |
| `IsAlive` | Check if agent process is running | runtime handle | alive boolean |
| `Heartbeat` | Health check | — | healthy, uptime |
| `Shutdown` | Request daemon shutdown | graceful flag | accepted boolean |

## Client Implementation

The orchestrator's `RemoteNode` client in `ennio-ssh` handles:
- SSH tunnel establishment to the gRPC port
- Connection management and reconnection
- All RPC calls with proper error mapping to `EnnioError`
- Health checking before operations

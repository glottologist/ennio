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
    ├── gRPC HealthCheck ──────────►│
    │◄──────────────── Serving ──────┤
    ├── gRPC CreateWorkspace ──────►│
    │◄──────────── workspace path ───┤
    ├── gRPC SpawnAgent ───────────►│
    │◄──────────── session id ───────┤
    │         ...polling...          │
    ├── gRPC Disconnect ───────────►│
    │◄──────────── shutdown ─────────┤
    │                                │
```

## Idle Timeout

The daemon automatically shuts down after `--idle-timeout` seconds of no gRPC activity. This prevents orphaned daemons from consuming resources on remote machines. The orchestrator re-deploys the daemon on next connection.

## Service Definition

The gRPC service is defined in `crates/ennio-proto/` using Protocol Buffers. The orchestrator's `RemoteNode` client in `ennio-ssh` handles connection establishment, health checking, and all RPC calls.

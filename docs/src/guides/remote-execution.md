# Remote Execution

Ennio can run agent sessions on remote machines over SSH. Projects with `ssh_config` in their configuration run remotely; projects without it run locally.

## How It Works

1. Ennio connects to the remote host via SSH (using `russh`)
2. Creates a workspace on the remote machine (worktree or clone)
3. Launches the agent inside a tmux/tmate session on the remote host
4. Monitors the session by reading tmux pane output over SSH
5. Optionally runs an `ennio-node` gRPC daemon for structured communication

## SSH Strategies

| Strategy | Description |
|----------|-------------|
| `tmux` | Creates a tmux session on the remote host. Most reliable. |
| `tmate` | Creates a tmate session for shared terminal access. |
| `remote_control` | Uses `claude remote-control` for direct agent control. |
| `node` | Deploys and communicates with `ennio-node` via gRPC over SSH tunnel. |

## Configuration

```yaml
projects:
  - name: remote-project
    repo: git@github.com:user/repo.git
    path: /home/deploy/repos/repo
    ssh_config:
      host: build-server.example.com
      port: 22
      username: deploy
      auth:
        type: key
        path: ~/.ssh/id_ed25519
      strategy: tmux
      connection_timeout: 30s
      keepalive_interval: 60s
      host_key_policy: accept_new
      known_hosts_path: ~/.ssh/known_hosts
```

## Authentication Methods

### SSH Key (default)

```yaml
ssh_config:
  auth:
    type: key
    path: ~/.ssh/id_ed25519
    passphrase: ${SSH_PASSPHRASE}   # optional
```

### SSH Agent

```yaml
ssh_config:
  auth:
    type: agent
```

Uses the running SSH agent (`SSH_AUTH_SOCK`).

### Password

```yaml
ssh_config:
  auth:
    type: password
    password: ${SSH_PASSWORD}
```

> **Warning:** Password authentication is less secure than key-based auth. Use it only when key-based auth is not available.

## Host Key Verification

| Policy | Behavior |
|--------|----------|
| `strict` | Only connect if host key matches `known_hosts` |
| `accept_new` | Accept and persist unknown keys, reject changed keys (default) |
| `accept_all` | Accept any host key (insecure, for testing only) |

## Remote Node Daemon

For structured communication beyond tmux send-keys, deploy `ennio-node` on the remote host:

```yaml
ssh_config:
  strategy: node
  node_config:
    enabled: true
    port: 9100
    idle_timeout: 3600
    workspace_root: /home/deploy/ennio-workspaces
    ennio_binary_path: /usr/local/bin/ennio-node
    auth_token: ${ENNIO_NODE_TOKEN}
```

The node daemon:
- Accepts gRPC calls over an SSH-tunneled port
- Manages workspaces and agent sessions locally on the remote host
- Reports health status back to the orchestrator
- Shuts down automatically after the idle timeout

### Node Management

```bash
ennio node list                    # list all configured node projects
ennio node status build-server     # check node health
ennio node connect remote-project  # establish connection
ennio node disconnect remote-project
```

## Local vs Remote Routing

The session manager routes automatically based on config:

| `ssh_config` present? | Workspace | Runtime | Communication |
|-----------------------|-----------|---------|---------------|
| No | Local worktree/clone | Local tmux/process | Direct |
| Yes, strategy: tmux | Remote via SSH | Remote tmux via SSH | SSH send-keys |
| Yes, strategy: node | Remote via gRPC | Remote via gRPC | SSH-tunneled gRPC |

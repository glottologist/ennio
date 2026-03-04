# Configuration Reference

Complete reference for `ennio.yaml`. All fields listed with types, defaults, and descriptions.

## Top-Level Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `3000` | Web API listen port |
| `terminal_port` | `u16` | `3001` | Terminal WebSocket port |
| `direct_terminal_port` | `u16?` | — | Direct terminal access port |
| `ready_threshold` | `Duration` | `2s` | Time before a session is considered ready |
| `defaults` | `DefaultPlugins` | see below | Global plugin defaults |
| `projects` | `[ProjectConfig]` | `[]` | List of project configurations |
| `notifiers` | `[NotifierConfig]` | `[]` | Notification channel definitions |
| `notification_routing` | `Map<String, [String]>` | `{}` | Route reaction types to specific notifiers |
| `reactions` | `Map<String, ReactionConfig>` | built-in set | Global reaction overrides |
| `database_url` | `String?` | `sqlite::memory:` | SQLite database URL |
| `nats_url` | `String?` | `nats://127.0.0.1:4222` | NATS server URL |
| `api_token` | `SecretString?` | — | Bearer token for API authentication |
| `cors_origins` | `[String]` | `[]` | Allowed CORS origins |

## DefaultPlugins

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `runtime` | `String` | `"tmux"` | Default runtime plugin |
| `agent` | `String` | `"claude-code"` | Default agent plugin |
| `workspace` | `String` | `"worktree"` | Default workspace plugin |
| `notifiers` | `[String]` | `[]` | Default notifier names |

## ProjectConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `String` | **required** | Unique project identifier |
| `project_id` | `ProjectId?` | — | Explicit project ID (auto-derived from name if omitted) |
| `repo` | `String` | **required** | Git repository URL |
| `path` | `PathBuf` | **required** | Absolute local path to the repository |
| `default_branch` | `String` | `"main"` | Default git branch |
| `session_prefix` | `String?` | — | Prefix for generated session IDs |
| `runtime` | `String?` | — | Override default runtime |
| `agent` | `String?` | — | Override default agent |
| `workspace` | `String?` | — | Override default workspace |
| `tracker_config` | `TrackerConfig?` | — | Issue tracker configuration |
| `scm_config` | `ScmConfig?` | — | Source control configuration |
| `symlinks` | `[SymlinkConfig]` | `[]` | Symlinks to create in workspace |
| `post_create` | `[String]` | `[]` | Shell commands run after workspace creation |
| `agent_config` | `AgentSpecificConfig?` | — | Agent-specific settings |
| `reactions` | `Map<String, ReactionConfig>` | `{}` | Project-specific reaction overrides |
| `agent_rules` | `[String]` | `[]` | Instructions passed to the agent |
| `max_sessions` | `u32?` | — | Maximum concurrent sessions |
| `ssh_config` | `SshConnectionConfig?` | — | Remote execution config (enables SSH mode) |

## TrackerConfig

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `String` | `"github"` or `"linear"` |
| `owner` | `String` | Repository/organization owner |
| `repo` | `String` | Repository name |
| `token` | `SecretString` | API token |

## ScmConfig

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `String` | `"github"` |
| `owner` | `String` | Repository owner |
| `repo` | `String` | Repository name |
| `token` | `SecretString` | API token |

## NotifierConfig

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Unique notifier identifier |
| `provider` | `String` | `"desktop"`, `"slack"`, or `"webhook"` |
| `webhook_url` | `String?` | Slack webhook URL (slack provider) |
| `url` | `String?` | Webhook URL (webhook provider) |

## ReactionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `action` | `ReactionAction` | **required** | `send_to_agent`, `notify`, or `auto_merge` |
| `message` | `String?` | — | Message to send (for `send_to_agent`) |
| `max_retries` | `u32?` | — | Maximum retry attempts |
| `escalation_timeout` | `u64?` | — | Seconds before escalating |
| `priority` | `EventPriority?` | — | Notification priority |

## AgentSpecificConfig

| Field | Type | Description |
|-------|------|-------------|
| `model` | `String?` | Model to use (e.g., `"opus"`, `"sonnet"`) |
| `max_turns` | `u32?` | Maximum conversation turns |

## SymlinkConfig

| Field | Type | Description |
|-------|------|-------------|
| `source` | `String` | Source path (relative to repo root) |
| `target` | `String` | Target path in workspace |

## SshConnectionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `String` | **required** | Remote hostname |
| `port` | `u16` | `22` | SSH port |
| `username` | `String` | **required** | SSH username |
| `auth` | `SshAuthConfig` | **required** | Authentication method |
| `strategy` | `SshStrategyConfig` | **required** | Remote execution strategy |
| `connection_timeout` | `Duration` | `30s` | SSH connection timeout |
| `keepalive_interval` | `Duration?` | — | SSH keepalive interval |
| `host_key_policy` | `HostKeyPolicyConfig` | `accept_new` | Host key verification policy |
| `known_hosts_path` | `PathBuf?` | — | Path to known_hosts file |
| `node_config` | `NodeConnectionConfig?` | — | Remote node daemon config |

### SshAuthConfig Variants

**Key authentication:**
```yaml
auth:
  type: key
  path: ~/.ssh/id_ed25519
  passphrase: ${SSH_PASSPHRASE}  # optional
```

**Agent authentication:**
```yaml
auth:
  type: agent
```

**Password authentication:**
```yaml
auth:
  type: password
  password: ${SSH_PASSWORD}
```

### HostKeyPolicyConfig

| Value | Description |
|-------|-------------|
| `strict` | Reject unknown or changed host keys |
| `accept_new` | Accept unknown keys, reject changed keys (default) |
| `accept_all` | Accept any key (insecure) |

## NodeConnectionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `9100` | gRPC listen port on remote host |
| `idle_timeout` | `Duration` | `3600s` | Auto-shutdown after idle |
| `workspace_root` | `PathBuf?` | — | Root directory for workspaces |
| `ennio_binary_path` | `PathBuf?` | — | Path to `ennio-node` binary on remote |
| `auth_token` | `SecretString?` | — | Bearer token for gRPC auth |

## Full Example

```yaml
port: 3000
terminal_port: 3001
api_token: ${ENNIO_API_TOKEN}
database_url: sqlite:ennio.db
nats_url: nats://127.0.0.1:4222
cors_origins:
  - http://localhost:3000

defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree
  notifiers:
    - desktop

notifiers:
  - name: desktop
    provider: desktop
  - name: team-slack
    provider: slack
    webhook_url: ${SLACK_WEBHOOK}

notification_routing:
  agent-exited:
    - team-slack
    - desktop
  all-complete:
    - desktop

reactions:
  approved-and-green:
    action: auto_merge

projects:
  - name: backend
    repo: git@github.com:org/backend.git
    path: /home/user/repos/backend
    default_branch: main
    max_sessions: 3
    session_prefix: be
    tracker_config:
      provider: github
      owner: org
      repo: backend
      token: ${GITHUB_TOKEN}
    scm_config:
      provider: github
      owner: org
      repo: backend
      token: ${GITHUB_TOKEN}
    symlinks:
      - source: ../.env
        target: .env
    post_create:
      - cargo build
    agent_config:
      model: opus
      max_turns: 200
    agent_rules:
      - "Write tests for all new code"
      - "Use conventional commits"

  - name: remote-ml
    repo: git@github.com:org/ml-pipeline.git
    path: /home/gpu-user/repos/ml-pipeline
    agent: aider
    ssh_config:
      host: gpu-server.internal
      username: gpu-user
      auth:
        type: key
        path: ~/.ssh/id_ed25519
      strategy: node
      node_config:
        port: 9100
        workspace_root: /home/gpu-user/workspaces
        auth_token: ${NODE_AUTH_TOKEN}
```

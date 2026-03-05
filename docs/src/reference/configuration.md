# Configuration Reference

Complete reference for `ennio.yaml`. All fields listed with types, defaults, and descriptions.

## Top-Level Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `3000` | Web API listen port |
| `terminal_port` | `u16` | `3001` | Terminal WebSocket port |
| `direct_terminal_port` | `u16?` | — | Direct terminal access port |
| `ready_threshold` | `Duration` | `2s` | Time before a session is considered ready (milliseconds in YAML) |
| `defaults` | `DefaultPlugins` | see below | Global plugin defaults |
| `projects` | `[ProjectConfig]` | `[]` | List of project configurations |
| `notifiers` | `[NotifierConfig]` | `[]` | Notification channel definitions |
| `notification_routing` | `Map<String, [String]>` | `{}` | Route reaction types to specific notifiers |
| `reactions` | `Map<String, ReactionConfig>` | built-in set | Global reaction overrides |
| `database_url` | `String?` | `sqlite:ennio.db` | SQLite database URL |
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
| `plugin` | `String` | Plugin name: `"github"` or `"linear"` |
| `config` | `Map<String, Value>` | Plugin-specific configuration (e.g., `owner`, `repo`, `token`) |

```yaml
tracker_config:
  plugin: github
  config:
    owner: my-org
    repo: my-repo
    token: ${GITHUB_TOKEN}
```

## ScmConfig

| Field | Type | Description |
|-------|------|-------------|
| `plugin` | `String` | Plugin name: `"github"` |
| `config` | `Map<String, Value>` | Plugin-specific configuration (e.g., `owner`, `repo`, `token`) |

```yaml
scm_config:
  plugin: github
  config:
    owner: my-org
    repo: my-repo
    token: ${GITHUB_TOKEN}
```

## NotifierConfig

| Field | Type | Description |
|-------|------|-------------|
| `plugin` | `String` | Plugin name: `"desktop"`, `"slack"`, or `"webhook"` |
| `name` | `String` | Unique notifier identifier (used in routing rules) |
| `config` | `Map<String, Value>` | Plugin-specific configuration |

```yaml
notifiers:
  - plugin: slack
    name: team-slack
    config:
      webhook_url: ${SLACK_WEBHOOK_URL}

  - plugin: webhook
    name: ops-alerts
    config:
      url: https://hooks.example.com/ennio

  - plugin: desktop
    name: local
    config: {}
```

## ReactionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Whether this reaction is active |
| `action` | `ReactionAction` | `"notify"` | `send_to_agent`, `notify`, or `auto_merge` |
| `message` | `String?` | — | Message to send (for `send_to_agent`) |
| `priority` | `EventPriority` | `"info"` | Notification priority level |
| `escalate_after` | `Duration?` | — | Seconds before escalating to notification |
| `threshold` | `Duration?` | — | Time threshold before reaction triggers (e.g., idle detection) |
| `retries` | `u32` | `0` | Maximum retry attempts |
| `include_summary` | `bool` | `false` | Include session summary in notification |

```yaml
reactions:
  ci-failed:
    enabled: true
    action: send_to_agent
    message: "CI failed. Check the logs and fix the issues."
    priority: action
    retries: 3
    escalate_after: 180
```

## AgentSpecificConfig

| Field | Type | Description |
|-------|------|-------------|
| `permissions` | `String?` | Permission mode for the agent (e.g., agent-specific flags) |
| `model` | `String?` | Model to use (e.g., `"opus"`, `"sonnet"`) |
| `passthrough` | `Map<String, Value>` | Additional key-value pairs passed through to the agent |

```yaml
agent_config:
  model: opus
  permissions: "--dangerously-skip-permissions"
  passthrough:
    max_turns: 200
```

## SymlinkConfig

| Field | Type | Description |
|-------|------|-------------|
| `source` | `PathBuf` | Source path (absolute or relative) |
| `target` | `PathBuf` | Target path in workspace |

## SshConnectionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `String` | **required** | Remote hostname |
| `port` | `u16` | `22` | SSH port |
| `username` | `String` | **required** | SSH username |
| `auth` | `SshAuthConfig` | **required** | Authentication method |
| `strategy` | `SshStrategyConfig` | `tmux` | Remote execution strategy |
| `connection_timeout` | `Duration` | `30s` | SSH connection timeout (seconds in YAML) |
| `keepalive_interval` | `Duration?` | — | SSH keepalive interval (seconds in YAML) |
| `host_key_policy` | `HostKeyPolicyConfig` | `strict` | Host key verification policy |
| `known_hosts_path` | `PathBuf?` | — | Path to known_hosts file |
| `node_config` | `NodeConnectionConfig?` | — | Remote node daemon config |

### SshAuthConfig Variants

Discriminated by the `type` field.

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

### SshStrategyConfig

| Value | Description |
|-------|-------------|
| `tmux` | Create a tmux session on the remote host (default) |
| `tmate` | Create a tmate session for shared terminal access |
| `remote_control` | Use agent's remote control protocol |
| `node` | Deploy and communicate via gRPC `ennio-node` daemon |

### HostKeyPolicyConfig

| Value | Description |
|-------|-------------|
| `strict` | Reject unknown or changed host keys (default) |
| `accept_new` | Accept unknown keys, reject changed keys |
| `accept_all` | Accept any key (insecure, for testing only) |

## NodeConnectionConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | `u16` | `9100` | gRPC listen port on remote host |
| `idle_timeout` | `Duration` | `3600s` | Auto-shutdown after idle (seconds in YAML) |
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
    - local

notifiers:
  - plugin: desktop
    name: local
    config: {}
  - plugin: slack
    name: team-slack
    config:
      webhook_url: ${SLACK_WEBHOOK}

notification_routing:
  agent-exited:
    - team-slack
    - local
  all-complete:
    - local

reactions:
  approved-and-green:
    enabled: true
    action: auto_merge

projects:
  - name: backend
    repo: git@github.com:org/backend.git
    path: /home/user/repos/backend
    default_branch: main
    max_sessions: 3
    session_prefix: be
    tracker_config:
      plugin: github
      config:
        owner: org
        repo: backend
        token: ${GITHUB_TOKEN}
    scm_config:
      plugin: github
      config:
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
      passthrough:
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
      host_key_policy: accept_new
      node_config:
        port: 9100
        workspace_root: /home/gpu-user/workspaces
        auth_token: ${NODE_AUTH_TOKEN}
```

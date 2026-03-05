# Notifications

Ennio sends notifications when reactions fire or sessions need attention.

## Notifier Plugins

| Plugin | Transport | Configuration |
|--------|-----------|--------------|
| `desktop` | `notify-send` (Linux) / `osascript` (macOS) | No config needed |
| `slack` | Slack incoming webhook | `webhook_url` |
| `webhook` | HTTP POST to any URL | `url` |

## Configuration

### Defining Notifiers

Each notifier specifies a `plugin` name, a unique `name` for routing, and a `config` map with plugin-specific settings:

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

### Default Notifiers

Set which notifiers are used by default:

```yaml
defaults:
  notifiers:
    - local
    - team-slack
```

### Notification Routing

Route specific reaction types to specific notifiers:

```yaml
notification_routing:
  ci-failed:
    - team-slack
  agent-exited:
    - team-slack
    - ops-alerts
  all-complete:
    - local
```

If no routing rule matches, the default notifiers are used.

## Event Priority

Notifications carry a priority from the triggering event:

| Priority | Meaning |
|----------|---------|
| `Info` | Status updates (e.g., all sessions complete) |
| `Action` | Something needs attention soon |
| `Urgent` | Immediate attention (e.g., agent exited, needs input) |
| `Critical` | System-level failure |

Notifier implementations can use priority to adjust formatting (e.g., Slack emoji, desktop urgency level).

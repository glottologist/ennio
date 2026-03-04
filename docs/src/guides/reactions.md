# Reactions

Reactions are configurable rules that fire when a session enters a specific state. They are the core of Ennio's autonomous behavior.

## How Reactions Work

1. The lifecycle manager detects a state change (e.g., CI failed)
2. It looks up matching reaction rules for the session's project
3. It checks the retry count and escalation timeout
4. It executes the action (send to agent, notify, auto-merge)
5. If the action fails or the state persists beyond the escalation timeout, it escalates

## Reaction Actions

| Action | Description |
|--------|-------------|
| `send_to_agent` | Sends a message to the running agent with instructions |
| `notify` | Sends a notification through configured notifier plugins |
| `auto_merge` | Merges the PR via the SCM plugin |

## Default Reactions

Ennio ships with 9 built-in reactions:

| Key | Trigger | Action | Retries | Escalation |
|-----|---------|--------|---------|------------|
| `ci-failed` | CI checks fail | `send_to_agent` | 2 | 120s |
| `changes-requested` | Reviewer requests changes | `send_to_agent` | — | 1800s |
| `bugbot-comments` | Bot comments on PR | `send_to_agent` | — | 1800s |
| `merge-conflicts` | Merge conflicts detected | `send_to_agent` | — | 900s |
| `approved-and-green` | PR approved + CI green | `notify` | — | — |
| `agent-stuck` | No activity for threshold | `notify` (urgent) | — | 600s |
| `agent-needs-input` | Agent waiting for input | `notify` (urgent) | — | — |
| `agent-exited` | Agent process exited | `notify` (urgent) | — | — |
| `all-complete` | All project sessions done | `notify` (info) | — | — |

## Configuration

Override reactions globally or per-project:

```yaml
# Global reactions
reactions:
  ci-failed:
    action: send_to_agent
    message: "CI failed. Check the logs and fix the issues."
    max_retries: 3
    escalation_timeout: 180

  approved-and-green:
    action: auto_merge   # auto-merge instead of just notifying

projects:
  - name: critical-app
    reactions:
      ci-failed:
        action: send_to_agent
        message: "CI is red. This is a critical service — fix immediately."
        max_retries: 5
        escalation_timeout: 60
```

Project-level reactions override global reactions for matching keys.

## Escalation

When a reaction's `escalation_timeout` expires and the state hasn't changed, Ennio escalates by sending a notification regardless of the original action. This ensures a human is alerted when automated recovery fails.

## Event Priority

Reactions emit events with a priority level that affects notification routing:

| Priority | Use |
|----------|-----|
| `Info` | Status updates, completions |
| `Action` | Something needs attention soon |
| `Urgent` | Immediate human attention required |
| `Critical` | System-level failures |

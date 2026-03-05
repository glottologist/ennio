# Quick Start

This guide gets you from zero to a running agent in under 5 minutes.

## 1. Initialize Configuration

```bash
ennio init .
```

This creates `ennio.yaml` with sensible defaults. Edit it to add your project:

```yaml
port: 3000
defaults:
  runtime: tmux
  agent: claude-code
  workspace: worktree

projects:
  - name: my-app
    repo: git@github.com:user/my-app.git
    path: /home/user/repos/my-app
    default_branch: main
    tracker_config:
      plugin: github
      config:
        owner: user
        repo: my-app
        token: ${GITHUB_TOKEN}
    scm_config:
      plugin: github
      config:
        owner: user
        repo: my-app
        token: ${GITHUB_TOKEN}
```

> **Note:** Environment variables in `${VAR}` syntax are expanded at load time.

## 2. Start the Orchestrator

```bash
ennio start
```

This boots the lifecycle polling loop, the web API, and connects to NATS and SQLite.

## 3. Spawn an Agent Session

```bash
# Work on a GitHub issue
ennio spawn my-app --issue 42

# Or give a direct prompt
ennio spawn my-app --prompt "Add input validation to the signup form"

# Optionally specify a branch
ennio spawn my-app --issue 42 --branch feat/signup-validation
```

Ennio will:
1. Create an isolated workspace (git worktree by default)
2. Run any `post_create` hooks (e.g., `npm install`)
3. Launch the agent with the issue description or prompt
4. Begin monitoring the session lifecycle

## 4. Monitor Progress

```bash
# See all sessions
ennio status

# Filter by project
ennio status my-app

# Detailed session info
ennio session info <session-id>

# Open the web dashboard
ennio dashboard
```

## 5. Interact with a Session

```bash
# Send additional instructions to the agent
ennio send <session-id> "Also add unit tests"

# Open the agent's tmux terminal
ennio open <session-id>

# Kill a stuck session
ennio session kill <session-id>

# Restore an exited session
ennio session restore <session-id>
```

## 6. Stop the Orchestrator

```bash
ennio stop
```

## What Happens Automatically

Once a session is spawned, the lifecycle manager polls external state and reacts:

| Event | Default Reaction |
|-------|-----------------|
| CI fails | Sends failure logs to the agent (up to 2 retries) |
| Code review requests changes | Sends review comments to the agent |
| Merge conflicts detected | Instructs the agent to rebase |
| PR approved + CI green | Notifies you (or auto-merges if configured) |
| Agent exits unexpectedly | Sends urgent notification |

All reactions are configurable per-project. See the [Reactions](../guides/reactions.md) guide.

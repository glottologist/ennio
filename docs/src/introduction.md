# Ennio

**Ennio** is an AI agent orchestrator, named after [Ennio Morricone](https://en.wikipedia.org/wiki/Ennio_Morricone) — the legendary Italian composer who conducted entire orchestras to create something greater than any single instrument could achieve.

Ennio does the same with AI coding agents. It spawns multiple agents across repositories, monitors their progress through the full pull request lifecycle, and reacts when things go wrong — automatically sending CI failure logs back to the agent, notifying you when reviews come in, and even auto-merging approved PRs.

## What Ennio Does

- **Spawns** AI coding agents (Claude Code, Aider, Codex, OpenCode) in isolated git worktrees or cloned workspaces
- **Monitors** each session through 16 lifecycle states, from `Spawning` through `Merged`
- **Reacts** to external events — CI failures, code review feedback, merge conflicts — by sending instructions back to the agent or notifying you
- **Runs locally or remotely** over SSH, with a gRPC daemon (`ennio-node`) for structured remote communication
- **Tracks costs** with double-entry bookkeeping and configurable budgets
- **Exposes** a REST API, web dashboard, and terminal UI for monitoring

## Key Concepts

| Concept | Description |
|---------|-------------|
| **Session** | A single agent working on a task in an isolated workspace |
| **Project** | A repository configuration with plugin overrides and reaction rules |
| **Plugin** | A swappable implementation for one of 7 slots (agent, runtime, workspace, tracker, SCM, notifier, terminal) |
| **Reaction** | A rule that fires when a session enters a specific state (e.g., CI failed → send logs to agent) |
| **Lifecycle Manager** | The polling loop that checks external state and triggers reactions |

## Architecture at a Glance

Ennio is a Rust workspace with 16 crates:

```
ennio-cli          CLI binary (clap)
ennio-core         Shared types, config, session model
ennio-services     Orchestration (session manager, lifecycle, event bus)
ennio-plugins      All plugin implementations
ennio-db           SQLite persistence
ennio-ssh          SSH client and remote strategies
ennio-node         Remote gRPC daemon
ennio-proto        Protobuf service definitions
ennio-web          REST API (axum)
ennio-tui          Terminal dashboard (ratatui)
ennio-dashboard    Web dashboard (dioxus WASM)
ennio-nats         NATS messaging
ennio-ledger       Cost tracking and budgets
ennio-ml           ML trait interfaces
ennio-observe      OpenTelemetry integration
ennio-doc          Documentation crate
```

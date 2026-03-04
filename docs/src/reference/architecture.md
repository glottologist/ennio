# Architecture Overview

## System Design

Ennio follows a modular, plugin-driven architecture. The core orchestration loop is decoupled from specific agent implementations, runtimes, and external services through trait-based plugin slots.

```
┌─────────────────────────────────────────────────────┐
│                    ennio-cli                         │
│              (clap CLI binary)                       │
├─────────────────────────────────────────────────────┤
│                  ennio-services                      │
│  ┌──────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │ SessionMgr   │  │ LifecycleMgr│  │ EventBus  │  │
│  │ (spawn/kill/ │  │ (poll/react/│  │ (broadcast│  │
│  │  restore)    │  │  escalate)  │  │  channel)  │  │
│  └──────┬───────┘  └──────┬──────┘  └─────┬─────┘  │
├─────────┼─────────────────┼────────────────┼────────┤
│         │           ennio-plugins          │         │
│  ┌──────┴──────────────────┴───────────────┴──────┐ │
│  │ Agent │ Runtime │ Workspace │ Tracker │ SCM │...│ │
│  └────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────┤
│  ennio-core    │  ennio-db   │  ennio-ssh           │
│  (types,       │  (SQLite    │  (SSH client,         │
│   config,      │   persist)  │   strategies)         │
│   traits)      │             │                       │
├────────────────┼─────────────┼───────────────────────┤
│  ennio-web     │  ennio-nats │  ennio-ledger         │
│  (REST API)    │  (messaging)│  (cost tracking)      │
└────────────────┴─────────────┴───────────────────────┘
```

## Data Flow

### Session Spawn

1. CLI or API receives spawn request
2. **SessionManager** resolves plugins for the project
3. **Workspace** plugin creates isolated working directory
4. **Tracker** plugin fetches issue details (if `--issue` provided)
5. **Runtime** plugin launches the agent
6. Session is persisted to SQLite and an event is emitted

### Lifecycle Polling

1. **LifecycleManager** iterates over active sessions
2. For each session, queries **Tracker** and **SCM** plugins for external state
3. Compares external state against current session status
4. Triggers status transitions and fires matching **reactions**
5. Events are emitted to the **EventBus** (in-memory broadcast) and persisted to SQLite
6. Events are optionally published to NATS for external consumers

### Event System

Events flow through two channels:

- **EventBus** — tokio broadcast channel for in-process subscribers (web API, lifecycle manager)
- **NATS** — external messaging for distributed setups (multiple orchestrator instances, external monitoring)
- **SQLite** — persistent event log for history and debugging

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Trait-based plugins | Swap implementations without changing orchestration logic |
| SQLite over PostgreSQL | Zero-config, single-file database, embedded in the binary |
| Tokio broadcast for events | Lock-free, multi-consumer, backpressure-aware |
| SSH via `russh` (pure Rust) | No dependency on system `libssh`, works cross-platform |
| `SecretString` for tokens | Prevents accidental logging of secrets |
| Constant-time token comparison | Prevents timing attacks on API auth |
| Git worktrees as default workspace | Fast creation, shared git objects, minimal disk usage |

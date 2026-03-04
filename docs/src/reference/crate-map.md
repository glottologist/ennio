# Crate Map

Ennio is a Rust workspace with 15 crates. This page describes each crate's purpose, dependencies, and public API surface.

## Dependency Graph

```
ennio-cli ──► ennio-services ──► ennio-core
    │              │                  ▲
    │              ├──► ennio-db ─────┘
    │              ├──► ennio-nats ───┘
    │              └──► ennio-plugins ──► ennio-ssh ──► ennio-core
    │                       │
    ├──► ennio-web ─────────┘
    ├──► ennio-tui
    └──► ennio-observe

ennio-node ──► ennio-proto ──► ennio-core
    │              │
    └──► ennio-ssh ┘

ennio-dashboard (standalone WASM, no workspace deps)
ennio-ledger ──► (standalone, rust_decimal)
ennio-ml ──► ennio-core
```

## Crate Details

### `ennio-core`

Shared foundation crate. No heavy dependencies.

- **Types**: `SessionId`, `ProjectId`, `EventId` (validated newtypes)
- **Config**: `OrchestratorConfig`, `ProjectConfig`, `SshConnectionConfig`
- **Session**: `Session`, `SessionStatus` (16 states), `ActivityState`
- **Events**: `OrchestratorEvent`, `EventType` (30 variants), `EventPriority`
- **Traits**: All 7 plugin traits (`Agent`, `Runtime`, `Workspace`, `Tracker`, `Scm`, `Notifier`, `Terminal`)
- **Paths**: Workspace path construction utilities

### `ennio-services`

Core orchestration logic.

- **SessionManager** — spawn, kill, restore, list, send
- **LifecycleManager** — poll loop, status transitions, reaction engine
- **EventBus** — tokio broadcast channel for in-process event distribution
- **ConfigLoader** — config file discovery and loading
- **PluginRegistry** — stores plugin instances for all 7 slots

### `ennio-plugins`

Concrete plugin implementations.

- 4 agent plugins, 3 runtime plugins, 2 workspace plugins
- 2 tracker plugins, 1 SCM plugin, 3 notifier plugins, 1 terminal plugin
- See [Plugin System](../guides/plugins.md) for the full list

### `ennio-db`

SQLite persistence layer.

- Connection pool with WAL journal mode
- SQL migrations (auto-run on startup)
- Repository functions: sessions, events, projects, metrics
- Uses `sqlx` with compile-time-unchecked queries

### `ennio-ssh`

SSH client and remote execution.

- **SshClient** — connect, execute commands, upload/download files
- **Strategies**: `TmuxStrategy`, `TmateStrategy`, `RemoteControlStrategy`
- **SshRuntime** — runtime plugin backed by SSH + strategy
- **RemoteNode** — gRPC client for `ennio-node` over SSH tunnel
- **Workspaces**: `SshWorktreeWorkspace`, `SshCloneWorkspace`

### `ennio-node`

Remote gRPC daemon binary.

- Runs on remote machines, managed by orchestrator over SSH
- Accepts workspace creation, agent spawn, and health check RPCs
- Bearer token authentication (optional)
- Auto-shutdown after idle timeout

### `ennio-proto`

Protobuf service definitions and generated code.

- `.proto` files defining the node service
- `tonic`/`prost` code generation via `build.rs`
- Type conversions between proto types and `ennio-core` types

### `ennio-web`

REST API server.

- Built on `axum`
- 5 authenticated endpoints + 1 health check
- Bearer token middleware
- CORS support

### `ennio-tui`

Terminal dashboard.

- Built on `ratatui` + `crossterm`
- Session table with color-coded status
- Detail panel and event log
- Keyboard-driven navigation

### `ennio-dashboard`

Web dashboard (Dioxus WASM).

- Session cards with status, activity, branch, PR info
- Attention zones for sessions needing action
- Standalone WASM binary (no workspace crate deps)

### `ennio-nats`

NATS messaging layer.

- Topic hierarchy builders and subscribe patterns
- Typed event publishing
- Client and subscription wrappers around `async-nats`

### `ennio-ledger`

Cost tracking and budgets.

- `Ledger` async trait with `InMemoryLedger` implementation
- Double-entry bookkeeping (accounts, transfers)
- Budget scopes (global/project/session) and periods (daily/monthly/total)
- `rust_decimal::Decimal` for monetary precision

### `ennio-ml`

ML trait interfaces (no implementations).

- `SessionOutcomePredictor` — predict success probability, duration, cost
- `AnomalyDetector` — detect anomalous metrics
- `CostPredictor` — estimate remaining and total cost
- Infrastructure-only: traits for future ML backing

### `ennio-observe`

OpenTelemetry integration.

- Tracing subscriber setup
- OTLP exporter configuration
- Prometheus metrics endpoint

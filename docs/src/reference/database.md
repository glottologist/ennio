# Database Schema

Ennio uses SQLite with WAL journal mode for persistence. The database is created automatically on startup and migrations run on every boot.

## Connection

```yaml
database_url: sqlite:ennio.db      # file-based (recommended)
database_url: sqlite::memory:       # in-memory (default, data lost on restart)
```

The connection pool is configured with:
- WAL journal mode (concurrent reads)
- Foreign keys enabled
- `SqlitePool` from `sqlx`

## Tables

### `sessions`

Stores session metadata and state.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| `id` | `TEXT PRIMARY KEY` | — | Session ID (e.g., `myapp-abc123`) |
| `project_id` | `TEXT NOT NULL` | — | Owning project |
| `status` | `TEXT NOT NULL` | `'spawning'` | Current `SessionStatus` |
| `activity` | `TEXT` | — | Current `ActivityState` |
| `branch` | `TEXT` | — | Git branch name |
| `issue_id` | `TEXT` | — | Linked issue ID |
| `workspace_path` | `TEXT` | — | Absolute path to workspace directory |
| `runtime_handle` | `TEXT` | — | JSON-serialized runtime handle |
| `agent_info` | `TEXT` | — | JSON-serialized agent session info |
| `agent_name` | `TEXT` | — | Agent plugin name |
| `pr_url` | `TEXT` | — | Pull request URL |
| `pr_number` | `INTEGER` | — | Pull request number |
| `tmux_name` | `TEXT` | — | Tmux session name |
| `config_hash` | `TEXT NOT NULL` | — | Hash of config at spawn time |
| `role` | `TEXT` | — | Session role |
| `metadata` | `TEXT NOT NULL` | `'{}'` | JSON metadata |
| `created_at` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 creation timestamp |
| `last_activity_at` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 last activity timestamp |
| `restored_at` | `TEXT` | — | ISO 8601 restore timestamp |
| `archived_at` | `TEXT` | — | ISO 8601 archive timestamp |

### `events`

Stores the event log.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| `id` | `TEXT PRIMARY KEY` | — | Event ID |
| `event_type` | `TEXT NOT NULL` | — | `EventType` variant name |
| `priority` | `TEXT NOT NULL` | — | `EventPriority` variant name |
| `session_id` | `TEXT NOT NULL` | — | Associated session (FK → sessions) |
| `project_id` | `TEXT NOT NULL` | — | Owning project |
| `timestamp` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 event timestamp |
| `message` | `TEXT NOT NULL` | — | Human-readable description |
| `data` | `TEXT NOT NULL` | `'{}'` | JSON payload |

### `projects`

Stores project metadata.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| `project_id` | `TEXT PRIMARY KEY` | — | Project ID |
| `name` | `TEXT NOT NULL` | — | Project name |
| `repo` | `TEXT NOT NULL` | — | Git repository URL |
| `path` | `TEXT NOT NULL` | — | Local filesystem path |
| `default_branch` | `TEXT NOT NULL` | `'main'` | Default git branch |
| `config_hash` | `TEXT NOT NULL` | — | Hash of project config |
| `created_at` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 creation timestamp |
| `updated_at` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 last update timestamp |

### `session_metrics`

Stores per-session performance metrics.

| Column | Type | Default | Description |
|--------|------|---------|-------------|
| `session_id` | `TEXT PRIMARY KEY` | — | Associated session (FK → sessions) |
| `total_tokens_in` | `INTEGER NOT NULL` | `0` | Total input tokens consumed |
| `total_tokens_out` | `INTEGER NOT NULL` | `0` | Total output tokens generated |
| `estimated_cost_usd` | `REAL NOT NULL` | `0.0` | Estimated total cost in USD |
| `ci_runs` | `INTEGER NOT NULL` | `0` | Number of CI runs |
| `ci_failures` | `INTEGER NOT NULL` | `0` | Number of CI failures |
| `review_rounds` | `INTEGER NOT NULL` | `0` | Number of review rounds |
| `time_to_first_pr_secs` | `INTEGER` | — | Seconds from spawn to first PR |
| `time_to_merge_secs` | `INTEGER` | — | Seconds from spawn to merge |
| `updated_at` | `TEXT NOT NULL` | `datetime('now')` | ISO 8601 last update timestamp |

## Indices

| Index | Table | Columns | Purpose |
|-------|-------|---------|---------|
| `idx_events_session_id` | `events` | `session_id` | Fast event lookup by session |
| `idx_events_event_type` | `events` | `event_type` | Fast event lookup by type |
| `idx_sessions_project_status` | `sessions` | `project_id, status` | Fast session filtering |

## Foreign Keys

- `events.session_id` → `sessions.id` (ON DELETE CASCADE)
- `session_metrics.session_id` → `sessions.id` (ON DELETE CASCADE)

## Migrations

Migrations are embedded in the binary and run automatically on startup. They are idempotent — using `CREATE TABLE IF NOT EXISTS` patterns. The migration order is:

1. **V1**: `sessions` table
2. **V2**: `events` table
3. **V3**: `projects` table
4. **V4**: `session_metrics` table
5. **V5**: Performance indices

## Querying

All database access goes through repository functions in `ennio-db`:

```
sessions::insert(pool, session)
sessions::get(pool, session_id) → Option<Session>
sessions::list(pool, project_filter) → Vec<Session>
sessions::update(pool, session)
sessions::delete(pool, session_id)

events::insert(pool, event)
events::get(pool, event_id) → Option<OrchestratorEvent>
events::list_by_session(pool, session_id) → Vec<OrchestratorEvent>
events::list_by_project(pool, project_id) → Vec<OrchestratorEvent>

projects::insert(pool, project)
projects::get(pool, project_id) → Option<ProjectRow>
projects::list(pool) → Vec<ProjectRow>

metrics::insert(pool, metrics)
metrics::get(pool, session_id) → Option<SessionMetricsRow>
metrics::update(pool, metrics)
```

All queries use parameterized bindings via `sqlx::query().bind()` — no SQL injection risk.

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

Stores session metadata.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `TEXT PRIMARY KEY` | Session ID (e.g., `myapp-abc123`) |
| `project_id` | `TEXT NOT NULL` | Owning project |
| `status` | `TEXT NOT NULL` | Current `SessionStatus` |
| `config_hash` | `TEXT` | Hash of config at spawn time |
| `branch` | `TEXT` | Git branch name |
| `pr_url` | `TEXT` | Pull request URL |
| `agent_name` | `TEXT` | Agent plugin name |
| `role` | `TEXT` | Session role |
| `created_at` | `TEXT NOT NULL` | ISO 8601 timestamp |
| `updated_at` | `TEXT NOT NULL` | ISO 8601 timestamp |

### `events`

Stores the event log.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `TEXT PRIMARY KEY` | Event ID |
| `session_id` | `TEXT NOT NULL` | Associated session |
| `event_type` | `TEXT NOT NULL` | `EventType` variant name |
| `priority` | `TEXT NOT NULL` | `EventPriority` variant name |
| `message` | `TEXT NOT NULL` | Human-readable description |
| `data` | `TEXT` | JSON payload |
| `created_at` | `TEXT NOT NULL` | ISO 8601 timestamp |

### `projects`

Stores project metadata.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `TEXT PRIMARY KEY` | Project ID |
| `name` | `TEXT NOT NULL` | Project name |
| `repo_url` | `TEXT NOT NULL` | Git repository URL |
| `config` | `TEXT` | JSON serialized config |
| `created_at` | `TEXT NOT NULL` | ISO 8601 timestamp |
| `updated_at` | `TEXT NOT NULL` | ISO 8601 timestamp |

### `metrics`

Stores session metrics.

| Column | Type | Description |
|--------|------|-------------|
| `id` | `INTEGER PRIMARY KEY` | Auto-increment ID |
| `session_id` | `TEXT NOT NULL` | Associated session |
| `metric_name` | `TEXT NOT NULL` | Metric key |
| `metric_value` | `REAL NOT NULL` | Numeric value |
| `created_at` | `TEXT NOT NULL` | ISO 8601 timestamp |

## Migrations

Migrations are embedded in the binary and run automatically on startup. They are idempotent — running them multiple times is safe. The migration system uses `CREATE TABLE IF NOT EXISTS` patterns.

## Querying

All database access goes through repository functions in `ennio-db`:

```
sessions::insert(pool, session)
sessions::get(pool, session_id) → Option<Session>
sessions::list(pool, project_filter) → Vec<Session>
sessions::update_status(pool, session_id, status)

events::insert(pool, event)
events::get(pool, event_id) → Option<OrchestratorEvent>
events::list_by_session(pool, session_id) → Vec<OrchestratorEvent>
```

All queries use parameterized bindings via `sqlx::query().bind()` — no SQL injection risk.

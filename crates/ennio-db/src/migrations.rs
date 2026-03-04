pub const V1_SESSIONS: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'spawning',
    activity TEXT,
    branch TEXT,
    issue_id TEXT,
    workspace_path TEXT,
    runtime_handle TEXT,
    agent_info TEXT,
    agent_name TEXT,
    pr_url TEXT,
    pr_number INTEGER,
    tmux_name TEXT,
    config_hash TEXT NOT NULL,
    role TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_activity_at TEXT NOT NULL DEFAULT (datetime('now')),
    restored_at TEXT,
    archived_at TEXT
);
"#;

pub const V2_EVENTS: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    priority TEXT NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    message TEXT NOT NULL,
    data TEXT NOT NULL DEFAULT '{}'
);
"#;

pub const V3_PROJECTS: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    repo TEXT NOT NULL,
    path TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    config_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub const V4_SESSION_METRICS: &str = r#"
CREATE TABLE IF NOT EXISTS session_metrics (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    total_tokens_in INTEGER NOT NULL DEFAULT 0,
    total_tokens_out INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
    ci_runs INTEGER NOT NULL DEFAULT 0,
    ci_failures INTEGER NOT NULL DEFAULT 0,
    review_rounds INTEGER NOT NULL DEFAULT 0,
    time_to_first_pr_secs INTEGER,
    time_to_merge_secs INTEGER,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub const V5A_IDX_EVENTS_SESSION: &str =
    "CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);";

pub const V5B_IDX_EVENTS_TYPE: &str =
    "CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);";

pub const V5C_IDX_SESSIONS_PROJECT: &str =
    "CREATE INDEX IF NOT EXISTS idx_sessions_project_status ON sessions(project_id, status);";

pub const ALL_MIGRATIONS: &[&str] = &[
    V1_SESSIONS,
    V2_EVENTS,
    V3_PROJECTS,
    V4_SESSION_METRICS,
    V5A_IDX_EVENTS_SESSION,
    V5B_IDX_EVENTS_TYPE,
    V5C_IDX_SESSIONS_PROJECT,
];

pub async fn run_all(pool: &sqlx::SqlitePool) -> Result<(), crate::error::DbError> {
    for migration in ALL_MIGRATIONS {
        sqlx::query(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_idempotency() {
        let pool = crate::pool::connect("sqlite::memory:").await.unwrap();
        run_all(&pool).await.unwrap();
        run_all(&pool).await.unwrap();
    }
}

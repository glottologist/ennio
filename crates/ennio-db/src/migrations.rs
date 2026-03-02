pub const V1_SESSIONS: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'spawning',
    activity TEXT,
    branch TEXT,
    issue_id TEXT,
    workspace_path TEXT,
    runtime_handle JSONB,
    agent_info JSONB,
    agent_name TEXT,
    pr_url TEXT,
    pr_number INTEGER,
    tmux_name TEXT,
    config_hash TEXT NOT NULL,
    role TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    restored_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ
);
"#;

pub const V2_EVENTS: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    priority TEXT NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    message TEXT NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
"#;

pub const V4_SESSION_METRICS: &str = r#"
CREATE TABLE IF NOT EXISTS session_metrics (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    total_tokens_in BIGINT NOT NULL DEFAULT 0,
    total_tokens_out BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    ci_runs INTEGER NOT NULL DEFAULT 0,
    ci_failures INTEGER NOT NULL DEFAULT 0,
    review_rounds INTEGER NOT NULL DEFAULT 0,
    time_to_first_pr_secs BIGINT,
    time_to_merge_secs BIGINT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
"#;

pub const V5_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_sessions_project_status ON sessions(project_id, status);
"#;

pub const ALL_MIGRATIONS: &[&str] = &[
    V1_SESSIONS,
    V2_EVENTS,
    V3_PROJECTS,
    V4_SESSION_METRICS,
    V5_INDEXES,
];

/// Run all migrations sequentially against the provided pool.
pub async fn run_all(pool: &sqlx::PgPool) -> Result<(), crate::error::DbError> {
    for migration in ALL_MIGRATIONS {
        sqlx::query(migration).execute(pool).await?;
    }
    Ok(())
}

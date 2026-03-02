use chrono::{DateTime, Utc};
use ennio_core::id::SessionId;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use crate::error::DbError;

#[derive(Debug, Clone)]
pub struct SessionMetricsRow {
    pub session_id: String,
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub estimated_cost_usd: f64,
    pub ci_runs: i32,
    pub ci_failures: i32,
    pub review_rounds: i32,
    pub time_to_first_pr_secs: Option<i64>,
    pub time_to_merge_secs: Option<i64>,
    pub updated_at: DateTime<Utc>,
}

fn map_metrics_row(row: &PgRow) -> Result<SessionMetricsRow, DbError> {
    Ok(SessionMetricsRow {
        session_id: row.try_get("session_id")?,
        total_tokens_in: row.try_get("total_tokens_in")?,
        total_tokens_out: row.try_get("total_tokens_out")?,
        estimated_cost_usd: row.try_get("estimated_cost_usd")?,
        ci_runs: row.try_get("ci_runs")?,
        ci_failures: row.try_get("ci_failures")?,
        review_rounds: row.try_get("review_rounds")?,
        time_to_first_pr_secs: row.try_get("time_to_first_pr_secs")?,
        time_to_merge_secs: row.try_get("time_to_merge_secs")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn upsert(
    pool: &PgPool,
    session_id: &SessionId,
    metrics: &SessionMetricsRow,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO session_metrics (
            session_id, total_tokens_in, total_tokens_out,
            estimated_cost_usd, ci_runs, ci_failures, review_rounds,
            time_to_first_pr_secs, time_to_merge_secs, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
        ON CONFLICT (session_id) DO UPDATE SET
            total_tokens_in = EXCLUDED.total_tokens_in,
            total_tokens_out = EXCLUDED.total_tokens_out,
            estimated_cost_usd = EXCLUDED.estimated_cost_usd,
            ci_runs = EXCLUDED.ci_runs,
            ci_failures = EXCLUDED.ci_failures,
            review_rounds = EXCLUDED.review_rounds,
            time_to_first_pr_secs = EXCLUDED.time_to_first_pr_secs,
            time_to_merge_secs = EXCLUDED.time_to_merge_secs,
            updated_at = NOW()
        "#,
    )
    .bind(session_id.as_str())
    .bind(metrics.total_tokens_in)
    .bind(metrics.total_tokens_out)
    .bind(metrics.estimated_cost_usd)
    .bind(metrics.ci_runs)
    .bind(metrics.ci_failures)
    .bind(metrics.review_rounds)
    .bind(metrics.time_to_first_pr_secs)
    .bind(metrics.time_to_merge_secs)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get(
    pool: &PgPool,
    session_id: &SessionId,
) -> Result<Option<SessionMetricsRow>, DbError> {
    let row = sqlx::query("SELECT * FROM session_metrics WHERE session_id = $1")
        .bind(session_id.as_str())
        .fetch_optional(pool)
        .await?;

    row.as_ref().map(map_metrics_row).transpose()
}

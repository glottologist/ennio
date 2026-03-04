use chrono::{DateTime, Utc};
use ennio_core::config::ProjectConfig;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::DbError;

#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub project_id: String,
    pub name: String,
    pub repo: String,
    pub path: String,
    pub default_branch: String,
    pub config_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn map_project_row(row: &SqliteRow) -> Result<ProjectRow, DbError> {
    Ok(ProjectRow {
        project_id: row.try_get("project_id")?,
        name: row.try_get("name")?,
        repo: row.try_get("repo")?,
        path: row.try_get("path")?,
        default_branch: row.try_get("default_branch")?,
        config_hash: row.try_get("config_hash")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn upsert(
    pool: &SqlitePool,
    project: &ProjectConfig,
    config_hash: &str,
) -> Result<(), DbError> {
    let project_id = project
        .project_id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .unwrap_or_else(|| project.name.replace(' ', "-").to_lowercase());

    let path_str = project.path.to_string_lossy();

    sqlx::query(
        r#"
        INSERT INTO projects (project_id, name, repo, path, default_branch, config_hash, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, datetime('now'))
        ON CONFLICT (project_id) DO UPDATE SET
            name = EXCLUDED.name,
            repo = EXCLUDED.repo,
            path = EXCLUDED.path,
            default_branch = EXCLUDED.default_branch,
            config_hash = EXCLUDED.config_hash,
            updated_at = datetime('now')
        "#,
    )
    .bind(&project_id)
    .bind(project.name.as_str())
    .bind(project.repo.as_str())
    .bind(path_str.as_ref())
    .bind(project.default_branch.as_str())
    .bind(config_hash)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get(pool: &SqlitePool, project_id: &str) -> Result<Option<ProjectRow>, DbError> {
    let row = sqlx::query("SELECT * FROM projects WHERE project_id = $1")
        .bind(project_id)
        .fetch_optional(pool)
        .await?;

    row.as_ref().map(map_project_row).transpose()
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<ProjectRow>, DbError> {
    let rows = sqlx::query("SELECT * FROM projects ORDER BY name ASC")
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_project_row).collect()
}

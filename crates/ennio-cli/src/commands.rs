use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing::{info, warn};

use ennio_core::config::OrchestratorConfig;
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::lifecycle::{LifecycleManager, SessionManager, SpawnRequest};
use ennio_nats::{EventPublisher, NatsClient};
use ennio_services::{
    DefaultLifecycleManager, DefaultSessionManager, EventBus, apply_project_defaults,
    find_config_file, load_config, register_default_plugins, validate_config,
};

use crate::format::{self, OutputFormat};

#[derive(Serialize)]
struct SessionSummary {
    id: String,
    project: String,
    status: String,
    agent: String,
    branch: String,
}

fn session_to_summary(s: &ennio_core::session::Session) -> SessionSummary {
    SessionSummary {
        id: s.id.to_string(),
        project: s.project_id.to_string(),
        status: s.status.to_string(),
        agent: s.agent_name.as_deref().unwrap_or("").to_owned(),
        branch: s.branch.as_deref().unwrap_or("").to_owned(),
    }
}

fn print_sessions(sessions: &[SessionSummary], output_format: &OutputFormat) -> Result<()> {
    match output_format {
        OutputFormat::Json => format::print_json(sessions)?,
        OutputFormat::Table => {
            if sessions.is_empty() {
                println!("No active sessions.");
            } else {
                format::print_table_header(&[
                    ("ID", 24),
                    ("PROJECT", 20),
                    ("STATUS", 16),
                    ("AGENT", 14),
                    ("BRANCH", 24),
                ]);
                for s in sessions {
                    format::print_table_row(&[
                        (&s.id, 24),
                        (&s.project, 20),
                        (&s.status, 16),
                        (&s.agent, 14),
                        (&s.branch, 24),
                    ]);
                }
            }
        }
    }
    Ok(())
}

fn load_orchestrator_config(config_path: Option<&str>) -> Result<OrchestratorConfig> {
    let path = match config_path {
        Some(p) => std::path::PathBuf::from(p),
        None => find_config_file(None).map_err(|e| anyhow::anyhow!("{e}"))?,
    };

    let mut config = load_config(&path).map_err(|e| anyhow::anyhow!("{e}"))?;

    validate_config(&config).map_err(|e| anyhow::anyhow!("{e}"))?;

    apply_project_defaults(&mut config);

    Ok(config)
}

async fn connect_db(config: &OrchestratorConfig) -> Result<SqlitePool> {
    let database_url = config.resolve_database_url();

    let pool = ennio_db::pool::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect to database: {e}"))?;

    ennio_db::migrations::run_all(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to run migrations: {e}"))?;

    Ok(pool)
}

async fn try_connect_nats(config: &OrchestratorConfig) -> Option<NatsClient> {
    if !config.nats_configured() {
        return None;
    }

    let nats_url = config.resolve_nats_url();
    match NatsClient::connect(&nats_url).await {
        Ok(client) => Some(client),
        Err(e) => {
            warn!("failed to connect to NATS at {nats_url}: {e}");
            None
        }
    }
}

fn make_publisher(nats_client: Option<NatsClient>) -> Arc<EventPublisher> {
    match nats_client {
        Some(client) => Arc::new(EventPublisher::new(client)),
        None => Arc::new(EventPublisher::without_nats()),
    }
}

struct ReadonlyBootstrap {
    pool: SqlitePool,
}

async fn bootstrap_readonly(config_path: Option<&str>) -> Result<ReadonlyBootstrap> {
    let config = load_orchestrator_config(config_path)?;
    let pool = connect_db(&config).await?;
    Ok(ReadonlyBootstrap { pool })
}

struct FullBootstrap {
    session_manager: Arc<dyn SessionManager>,
}

async fn bootstrap_full(config_path: Option<&str>) -> Result<FullBootstrap> {
    let config = load_orchestrator_config(config_path)?;
    let pool = connect_db(&config).await?;
    let nats_client = try_connect_nats(&config).await;
    let publisher = make_publisher(nats_client);
    let config = Arc::new(config);

    let registry = register_default_plugins(&config)
        .map_err(|e| anyhow::anyhow!("failed to register plugins: {e}"))?;
    let registry = Arc::new(registry);
    let event_bus = Arc::new(EventBus::new());

    let session_manager: Arc<dyn SessionManager> = Arc::new(DefaultSessionManager::new(
        Arc::clone(&registry),
        Arc::clone(&event_bus),
        Arc::clone(&config),
        pool,
        Arc::clone(&publisher),
    ));

    Ok(FullBootstrap { session_manager })
}

pub async fn init(path: &str) -> Result<()> {
    let dir = Path::new(path);
    let config_path = dir.join("ennio.yaml");

    if config_path.exists() {
        bail!("Config file already exists: {}", config_path.display());
    }

    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;

    let config = OrchestratorConfig::default();
    let yaml = serde_yaml::to_string(&config).context("failed to serialize default config")?;

    std::fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write config: {}", config_path.display()))?;

    println!("Created config: {}", config_path.display());
    Ok(())
}

struct ServerBootstrap {
    config: Arc<OrchestratorConfig>,
    session_manager: Arc<dyn SessionManager>,
    lifecycle_manager: Arc<DefaultLifecycleManager>,
    nats_client: Option<NatsClient>,
}

async fn bootstrap_server(config_path: Option<&str>) -> Result<ServerBootstrap> {
    let config = load_orchestrator_config(config_path)?;
    let pool = connect_db(&config).await?;
    let nats_client = try_connect_nats(&config).await;
    let publisher = make_publisher(nats_client.clone()); // clone: NatsClient is cheap (Arc internally), Option clone is fine
    let config = Arc::new(config);

    if config.expose_api_token().is_none() {
        warn!("no api_token configured — all API requests will be rejected with 401");
    }

    let registry = register_default_plugins(&config)
        .map_err(|e| anyhow::anyhow!("failed to register plugins: {e}"))?;
    let registry = Arc::new(registry);
    let event_bus = Arc::new(EventBus::new());

    let session_manager: Arc<dyn SessionManager> = Arc::new(DefaultSessionManager::new(
        Arc::clone(&registry),
        Arc::clone(&event_bus),
        Arc::clone(&config),
        pool.clone(), // clone: SqlitePool uses Arc internally
        Arc::clone(&publisher),
    ));

    let lifecycle_manager = Arc::new(DefaultLifecycleManager::new(
        Arc::clone(&registry),
        Arc::clone(&event_bus),
        Arc::clone(&config),
        pool, // clone: not needed — last use of pool
        Arc::clone(&publisher),
        Arc::clone(&session_manager),
    ));

    Ok(ServerBootstrap {
        config,
        session_manager,
        lifecycle_manager,
        nats_client,
    })
}

async fn run_server(
    config: &OrchestratorConfig,
    session_manager: &Arc<dyn SessionManager>,
    lifecycle_manager: &Arc<DefaultLifecycleManager>,
    nats_client: Option<NatsClient>,
) -> Result<()> {
    let web_state = Arc::new(ennio_web::state::AppState {
        session_manager: Arc::clone(session_manager),
        lifecycle_manager: lifecycle_manager.clone() as Arc<dyn LifecycleManager>, // clone: Arc ref count bump for trait object
        api_token: config.expose_api_token().map(str::to_owned),
        cors_origins: config.cors_origins.clone(), // clone: small Vec<String> for web state
    });

    let router = ennio_web::router::create_router(web_state);

    let bind_addr = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind to {bind_addr}"))?;

    info!(port = config.port, "ennio orchestrator started");
    println!("Ennio orchestrator listening on {bind_addr}");

    match nats_client {
        Some(client) => {
            let mut shutdown_sub = client
                .subscribe("ennio.commands.shutdown")
                .await
                .map_err(|e| anyhow::anyhow!("failed to subscribe to shutdown topic: {e}"))?;

            tokio::select! {
                result = axum::serve(listener, router) => {
                    if let Err(e) = result {
                        tracing::error!("web server error: {e}");
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("received ctrl-c, shutting down");
                }
                _ = shutdown_sub.next() => {
                    info!("received shutdown command via NATS");
                }
            }
        }
        None => {
            warn!("NATS not configured, remote shutdown unavailable (use Ctrl+C)");

            tokio::select! {
                result = axum::serve(listener, router) => {
                    if let Err(e) = result {
                        tracing::error!("web server error: {e}");
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("received ctrl-c, shutting down");
                }
            }
        }
    }

    Ok(())
}

pub async fn start(config_path: Option<&str>) -> Result<()> {
    let boot = bootstrap_server(config_path).await?;

    boot.lifecycle_manager
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("failed to start lifecycle manager: {e}"))?;

    let poll_lifecycle = Arc::clone(&boot.lifecycle_manager);
    let poll_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = poll_lifecycle.poll_sessions().await {
                tracing::warn!("poll_sessions error: {e}");
            }
        }
    });

    run_server(
        &boot.config,
        &boot.session_manager,
        &boot.lifecycle_manager,
        boot.nats_client,
    )
    .await?;

    poll_handle.abort();

    if let Err(e) = boot.lifecycle_manager.stop().await {
        tracing::warn!("lifecycle manager stop error: {e}");
    }

    info!("ennio orchestrator stopped");
    println!("Ennio orchestrator stopped.");
    Ok(())
}

pub async fn stop(config_path: Option<&str>) -> Result<()> {
    let config = load_orchestrator_config(config_path)?;

    if !config.nats_configured() {
        bail!(
            "NATS is not configured. Cannot send remote shutdown command.\n\
             Use Ctrl+C to stop a locally running orchestrator, or set nats_url in your config."
        );
    }

    let nats_client = try_connect_nats(&config)
        .await
        .ok_or_else(|| anyhow::anyhow!("failed to connect to NATS — is the server running?"))?;

    let publisher = EventPublisher::new(nats_client);

    publisher
        .publish_command("shutdown", &serde_json::json!({"reason": "cli-stop"}))
        .await
        .map_err(|e| anyhow::anyhow!("failed to send shutdown command: {e}"))?;

    println!("Shutdown command sent.");
    Ok(())
}

pub async fn status(
    project: Option<&str>,
    config_path: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let bootstrap = bootstrap_readonly(config_path).await?;

    let project_id = project
        .map(|p| ProjectId::new(p.to_owned()))
        .transpose()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let sessions = ennio_db::repo::sessions::list(&bootstrap.pool, project_id.as_ref())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list sessions: {e}"))?;

    let summaries: Vec<SessionSummary> = sessions.iter().map(session_to_summary).collect();
    print_sessions(&summaries, output_format)
}

pub async fn spawn(
    project: &str,
    issue: Option<&str>,
    prompt: Option<&str>,
    branch: Option<&str>,
    role: Option<&str>,
    config_path: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let bootstrap = bootstrap_full(config_path).await?;

    let project_id = ProjectId::new(project.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let request = SpawnRequest {
        project_id: &project_id,
        issue_id: issue,
        prompt,
        branch,
        role,
    };

    let session = bootstrap
        .session_manager
        .spawn(&request)
        .await
        .map_err(|e| anyhow::anyhow!("spawn failed: {e}"))?;

    let summary = session_to_summary(&session);

    match output_format {
        OutputFormat::Json => format::print_json(&summary)?,
        OutputFormat::Table => {
            println!("Session spawned: {}", summary.id);
            println!("  Project: {}", summary.project);
            println!("  Status:  {}", summary.status);
            println!("  Agent:   {}", summary.agent);
            if !summary.branch.is_empty() {
                println!("  Branch:  {}", summary.branch);
            }
        }
    }
    Ok(())
}

pub async fn session_info(
    id: &str,
    config_path: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let bootstrap = bootstrap_readonly(config_path).await?;

    let session_id = SessionId::new(id.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let session = ennio_db::repo::sessions::get(&bootstrap.pool, &session_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get session: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("session not found: {id}"))?;

    let summary = session_to_summary(&session);

    match output_format {
        OutputFormat::Json => format::print_json(&summary)?,
        OutputFormat::Table => {
            println!("Session: {}", summary.id);
            println!("  Project: {}", summary.project);
            println!("  Status:  {}", summary.status);
            println!("  Agent:   {}", summary.agent);
            if !summary.branch.is_empty() {
                println!("  Branch:  {}", summary.branch);
            }
        }
    }
    Ok(())
}

pub async fn session_kill(id: &str, config_path: Option<&str>) -> Result<()> {
    let bootstrap = bootstrap_full(config_path).await?;

    let session_id = SessionId::new(id.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    bootstrap
        .session_manager
        .kill(&session_id)
        .await
        .map_err(|e| anyhow::anyhow!("kill failed: {e}"))?;

    println!("Session killed: {id}");
    Ok(())
}

pub async fn session_restore(
    id: &str,
    config_path: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let bootstrap = bootstrap_full(config_path).await?;

    let session_id = SessionId::new(id.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let session = bootstrap
        .session_manager
        .restore(&session_id)
        .await
        .map_err(|e| anyhow::anyhow!("restore failed: {e}"))?;

    let summary = session_to_summary(&session);

    match output_format {
        OutputFormat::Json => format::print_json(&summary)?,
        OutputFormat::Table => {
            println!("Session restored: {}", summary.id);
            println!("  Status: {}", summary.status);
        }
    }
    Ok(())
}

pub async fn send(session: &str, message: &str, config_path: Option<&str>) -> Result<()> {
    let bootstrap = bootstrap_full(config_path).await?;

    let session_id = SessionId::new(session.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    bootstrap
        .session_manager
        .send(&session_id, message)
        .await
        .map_err(|e| anyhow::anyhow!("send failed: {e}"))?;

    println!("Message sent to session {session}.");
    Ok(())
}

pub async fn dashboard(port: u16) -> Result<()> {
    println!("Dashboard available at http://localhost:{port}");
    println!("Start the orchestrator with `ennio start` to enable the web API.");
    Ok(())
}

pub async fn open(session: &str, config_path: Option<&str>) -> Result<()> {
    let bootstrap = bootstrap_readonly(config_path).await?;

    let session_id = SessionId::new(session.to_owned()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let session_data = ennio_db::repo::sessions::get(&bootstrap.pool, &session_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to get session: {e}"))?
        .ok_or_else(|| anyhow::anyhow!("session not found: {session}"))?;

    match session_data.tmux_name {
        Some(ref name) => {
            println!("Attach to session terminal with:");
            println!("  tmux attach-session -t {name}");
        }
        None => {
            println!("No tmux session associated with {session}.");
        }
    }
    Ok(())
}

pub async fn node_status(_host: Option<&str>) -> Result<()> {
    Err(anyhow::anyhow!("node status is not yet implemented"))
}

pub async fn node_list() -> Result<()> {
    Err(anyhow::anyhow!("node list is not yet implemented"))
}

pub async fn node_connect(_project: &str) -> Result<()> {
    Err(anyhow::anyhow!("node connect is not yet implemented"))
}

pub async fn node_disconnect(_project: &str) -> Result<()> {
    Err(anyhow::anyhow!("node disconnect is not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn init_creates_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        init(path).await.unwrap();

        let config_path = dir.path().join("ennio.yaml");
        assert!(config_path.exists());

        let contents = std::fs::read_to_string(&config_path).unwrap();
        let parsed: ennio_core::config::OrchestratorConfig =
            serde_yaml::from_str(&contents).unwrap();
        assert_eq!(parsed.port, 3000);
        assert_eq!(parsed.projects.len(), 1);
    }

    #[tokio::test]
    async fn init_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        init(path).await.unwrap();

        let result = init(path).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("already exists"),
            "expected 'already exists' in: {err_msg}"
        );
    }
}

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::format::{self, OutputFormat};

pub async fn init(path: &str) -> Result<()> {
    let dir = Path::new(path);
    let config_path = dir.join("ennio.yaml");

    if config_path.exists() {
        bail!("Config file already exists: {}", config_path.display());
    }

    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create directory: {}", dir.display()))?;

    let config = ennio_core::config::OrchestratorConfig::default();
    let yaml = serde_yaml::to_string(&config).context("failed to serialize default config")?;

    std::fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write config: {}", config_path.display()))?;

    println!("Created config: {}", config_path.display());
    Ok(())
}

pub async fn start(_config: Option<&str>) -> Result<()> {
    println!("Starting ennio orchestrator...");
    Ok(())
}

pub async fn stop() -> Result<()> {
    println!("Stopping ennio orchestrator...");
    Ok(())
}

#[derive(Serialize)]
struct SessionSummary {
    id: String,
    project: String,
    status: String,
    agent: String,
    branch: String,
}

pub async fn status(_project: Option<&str>, output_format: &OutputFormat) -> Result<()> {
    let sessions: Vec<SessionSummary> = vec![];

    match output_format {
        OutputFormat::Json => format::print_json(&sessions)?,
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
                for s in &sessions {
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

pub async fn spawn(
    project: &str,
    _issue: Option<&str>,
    _prompt: Option<&str>,
    _branch: Option<&str>,
    _role: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let session = SessionSummary {
        id: "pending".to_owned(),
        project: project.to_owned(),
        status: "spawning".to_owned(),
        agent: "claude-code".to_owned(),
        branch: String::new(),
    };

    match output_format {
        OutputFormat::Json => format::print_json(&session)?,
        OutputFormat::Table => {
            println!("Spawning session for project: {project}");
        }
    }
    Ok(())
}

pub async fn session_info(id: &str, output_format: &OutputFormat) -> Result<()> {
    let session = SessionSummary {
        id: id.to_owned(),
        project: String::new(),
        status: "unknown".to_owned(),
        agent: String::new(),
        branch: String::new(),
    };

    match output_format {
        OutputFormat::Json => format::print_json(&session)?,
        OutputFormat::Table => {
            println!("Session: {id}");
            println!("  Status: unknown");
        }
    }
    Ok(())
}

pub async fn session_kill(id: &str) -> Result<()> {
    println!("Killing session: {id}");
    Ok(())
}

pub async fn session_restore(id: &str, output_format: &OutputFormat) -> Result<()> {
    let session = SessionSummary {
        id: id.to_owned(),
        project: String::new(),
        status: "restoring".to_owned(),
        agent: String::new(),
        branch: String::new(),
    };

    match output_format {
        OutputFormat::Json => format::print_json(&session)?,
        OutputFormat::Table => {
            println!("Restoring session: {id}");
        }
    }
    Ok(())
}

pub async fn send(session: &str, message: &str) -> Result<()> {
    println!("Sending to {session}: {message}");
    Ok(())
}

pub async fn dashboard(port: u16) -> Result<()> {
    println!("Starting dashboard on port {port}...");
    Ok(())
}

pub async fn open(session: &str) -> Result<()> {
    println!("Opening session terminal: {session}");
    Ok(())
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

    #[test]
    fn default_config_yaml_has_expected_structure() {
        let config = ennio_core::config::OrchestratorConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();

        assert!(yaml.contains("port: 3000"), "missing port");
        assert!(
            yaml.contains("terminal_port: 3001"),
            "missing terminal_port"
        );
        assert!(
            yaml.contains("ready_threshold: 2000"),
            "missing ready_threshold"
        );
        assert!(yaml.contains("runtime: tmux"), "missing runtime");
        assert!(yaml.contains("agent: claude-code"), "missing agent");
        assert!(yaml.contains("workspace: worktree"), "missing workspace");
        assert!(yaml.contains("name: my-project"), "missing project name");
        assert!(
            yaml.contains("default_branch: main"),
            "missing default_branch"
        );
        assert!(yaml.contains("ci-failed:"), "missing ci-failed reaction");
        assert!(
            yaml.contains("all-complete:"),
            "missing all-complete reaction"
        );
        assert!(
            !yaml.contains("direct_terminal_port"),
            "None fields should be skipped"
        );
        assert!(
            !yaml.contains("notifiers:"),
            "empty Vec fields should be skipped"
        );
        assert!(
            !yaml.contains("notification_routing"),
            "empty HashMap fields should be skipped"
        );
    }
}

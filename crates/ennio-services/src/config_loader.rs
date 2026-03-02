use std::path::{Path, PathBuf};

use ennio_core::config::OrchestratorConfig;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;

const CONFIG_FILE_NAMES: &[&str] = &["ennio.yaml", "ennio.yml", ".ennio.yaml", ".ennio.yml"];

pub fn find_config_file(start_dir: Option<&Path>) -> Result<PathBuf, EnnioError> {
    let start = match start_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir().map_err(|e| EnnioError::Config {
            message: format!("failed to get current directory: {e}"),
        })?,
    };

    let canonical_start = start.canonicalize().map_err(|e| EnnioError::Config {
        message: format!(
            "failed to canonicalize start directory {}: {e}",
            start.display()
        ),
    })?;

    let mut current = canonical_start.as_path();
    loop {
        for name in CONFIG_FILE_NAMES {
            let candidate = current.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Err(EnnioError::Config {
        message: format!(
            "no config file found (searched from {})",
            canonical_start.display()
        ),
    })
}

pub fn load_config(path: &Path) -> Result<OrchestratorConfig, EnnioError> {
    let canonical = path.canonicalize().map_err(|e| EnnioError::Io {
        path: Some(path.to_path_buf()),
        source: e,
    })?;

    let content = std::fs::read_to_string(&canonical).map_err(|e| EnnioError::Io {
        path: Some(canonical.clone()),
        source: e,
    })?;

    let config: OrchestratorConfig =
        serde_yaml::from_str(&content).map_err(|e| EnnioError::Config {
            message: format!("failed to parse {}: {e}", canonical.display()),
        })?;

    Ok(config)
}

pub fn validate_config(config: &OrchestratorConfig) -> Result<(), EnnioError> {
    if config.projects.is_empty() {
        return Err(EnnioError::Config {
            message: "config must define at least one project".to_owned(),
        });
    }

    for (idx, project) in config.projects.iter().enumerate() {
        if project.name.is_empty() {
            return Err(EnnioError::Config {
                message: format!("project at index {idx} has an empty name"),
            });
        }

        if project.repo.is_empty() {
            return Err(EnnioError::Config {
                message: format!("project '{}' has an empty repo", project.name),
            });
        }

        if !project.path.is_absolute() {
            return Err(EnnioError::Config {
                message: format!(
                    "project '{}' path must be absolute: {}",
                    project.name,
                    project.path.display()
                ),
            });
        }
    }

    for notifier in &config.notifiers {
        if notifier.name.is_empty() {
            return Err(EnnioError::Config {
                message: "notifier has an empty name".to_owned(),
            });
        }
        if notifier.plugin.is_empty() {
            return Err(EnnioError::Config {
                message: format!("notifier '{}' has an empty plugin", notifier.name),
            });
        }
    }

    Ok(())
}

pub fn apply_project_defaults(config: &mut OrchestratorConfig) {
    for project in &mut config.projects {
        if project.project_id.is_none() {
            let id_str = project.name.replace(' ', "-").to_lowercase();
            if let Ok(id) = ProjectId::new(id_str) {
                project.project_id = Some(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ennio_core::config::{DefaultPlugins, ProjectConfig};
    use rstest::rstest;

    use super::*;

    fn minimal_project(name: &str, path: &str) -> ProjectConfig {
        ProjectConfig {
            name: name.to_owned(),
            project_id: None,
            repo: "https://github.com/test/repo".to_owned(),
            path: PathBuf::from(path),
            default_branch: "main".to_owned(),
            session_prefix: None,
            runtime: None,
            agent: None,
            workspace: None,
            tracker_config: None,
            scm_config: None,
            symlinks: vec![],
            post_create: vec![],
            agent_config: None,
            reactions: std::collections::HashMap::new(),
            agent_rules: vec![],
            max_sessions: None,
        }
    }

    fn minimal_config(projects: Vec<ProjectConfig>) -> OrchestratorConfig {
        OrchestratorConfig {
            port: 3000,
            terminal_port: 3001,
            direct_terminal_port: None,
            ready_threshold: std::time::Duration::from_secs(2),
            defaults: DefaultPlugins::default(),
            projects,
            notifiers: vec![],
            notification_routing: std::collections::HashMap::new(),
            reactions: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn validate_empty_projects_rejected() {
        let config = minimal_config(vec![]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn validate_empty_project_name_rejected() {
        let config = minimal_config(vec![minimal_project("", "/tmp/test")]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn validate_empty_repo_rejected() {
        let mut project = minimal_project("test", "/tmp/test");
        project.repo = String::new();
        let config = minimal_config(vec![project]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn validate_relative_path_rejected() {
        let config = minimal_config(vec![minimal_project("test", "relative/path")]);
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn validate_valid_config_accepted() {
        let config = minimal_config(vec![minimal_project("test", "/tmp/test")]);
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn apply_defaults_generates_project_ids() {
        let mut config = minimal_config(vec![
            minimal_project("My Project", "/tmp/test1"),
            minimal_project("another", "/tmp/test2"),
        ]);

        assert!(config.projects[0].project_id.is_none());
        assert!(config.projects[1].project_id.is_none());

        apply_project_defaults(&mut config);

        assert_eq!(
            config.projects[0].project_id.as_ref().unwrap().as_str(),
            "my-project"
        );
        assert_eq!(
            config.projects[1].project_id.as_ref().unwrap().as_str(),
            "another"
        );
    }

    #[test]
    fn apply_defaults_preserves_existing_ids() {
        let mut project = minimal_project("test", "/tmp/test");
        project.project_id = Some(ProjectId::new("custom-id").unwrap());
        let mut config = minimal_config(vec![project]);

        apply_project_defaults(&mut config);

        assert_eq!(
            config.projects[0].project_id.as_ref().unwrap().as_str(),
            "custom-id"
        );
    }

    #[test]
    fn find_config_nonexistent_dir_errors() {
        let result = find_config_file(Some(Path::new("/nonexistent/path/that/does/not/exist")));
        assert!(result.is_err());
    }

    #[rstest]
    #[case("ennio.yaml")]
    #[case("ennio.yml")]
    #[case(".ennio.yaml")]
    #[case(".ennio.yml")]
    fn config_file_names_are_recognized(#[case] name: &str) {
        assert!(CONFIG_FILE_NAMES.contains(&name));
    }
}

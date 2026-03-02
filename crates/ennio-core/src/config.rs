use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::event::EventPriority;
use crate::id::ProjectId;
use crate::reaction::{ReactionAction, ReactionConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_terminal_port")]
    pub terminal_port: u16,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_terminal_port: Option<u16>,
    #[serde(default = "default_ready_threshold")]
    #[serde(with = "duration_millis")]
    pub ready_threshold: Duration,
    pub defaults: DefaultPlugins,
    pub projects: Vec<ProjectConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notifiers: Vec<NotifierConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub notification_routing: HashMap<String, Vec<String>>,
    #[serde(default = "default_reactions")]
    pub reactions: HashMap<String, ReactionConfig>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            terminal_port: default_terminal_port(),
            direct_terminal_port: None,
            ready_threshold: default_ready_threshold(),
            defaults: DefaultPlugins::default(),
            projects: vec![ProjectConfig::default()],
            notifiers: vec![],
            notification_routing: HashMap::new(),
            reactions: default_reactions(),
        }
    }
}

const fn default_port() -> u16 {
    3000
}

const fn default_terminal_port() -> u16 {
    3001
}

const fn default_ready_threshold() -> Duration {
    Duration::from_secs(2)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultPlugins {
    #[serde(default = "default_runtime")]
    pub runtime: String,
    #[serde(default = "default_agent")]
    pub agent: String,
    #[serde(default = "default_workspace")]
    pub workspace: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notifiers: Vec<String>,
}

fn default_runtime() -> String {
    "tmux".to_string()
}

fn default_agent() -> String {
    "claude-code".to_string()
}

fn default_workspace() -> String {
    "worktree".to_string()
}

impl Default for DefaultPlugins {
    fn default() -> Self {
        Self {
            runtime: default_runtime(),
            agent: default_agent(),
            workspace: default_workspace(),
            notifiers: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub repo: String,
    pub path: PathBuf,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_prefix: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_config: Option<TrackerConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scm_config: Option<ScmConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symlinks: Vec<SymlinkConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub post_create: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_config: Option<AgentSpecificConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub reactions: HashMap<String, ReactionConfig>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agent_rules: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_sessions: Option<u32>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "my-project".to_string(),
            project_id: None,
            repo: "https://github.com/owner/repo".to_string(),
            path: PathBuf::from("/absolute/path/to/project"),
            default_branch: default_branch(),
            session_prefix: None,
            runtime: None,
            agent: None,
            workspace: None,
            tracker_config: None,
            scm_config: None,
            symlinks: vec![],
            post_create: vec![],
            agent_config: None,
            reactions: HashMap::new(),
            agent_rules: vec![],
            max_sessions: None,
        }
    }
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerConfig {
    pub plugin: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmConfig {
    pub plugin: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifierConfig {
    pub plugin: String,
    pub name: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSpecificConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub passthrough: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymlinkConfig {
    pub source: PathBuf,
    pub target: PathBuf,
}

type ReactionDef = (
    &'static str,
    ReactionAction,
    EventPriority,
    u32,
    Option<u64>,
    Option<u64>,
    bool,
);

pub fn default_reactions() -> HashMap<String, ReactionConfig> {
    let defs: &[ReactionDef] = &[
        (
            "ci-failed",
            ReactionAction::SendToAgent,
            EventPriority::Action,
            2,
            Some(120),
            None,
            false,
        ),
        (
            "changes-requested",
            ReactionAction::SendToAgent,
            EventPriority::Action,
            0,
            Some(1800),
            None,
            false,
        ),
        (
            "bugbot-comments",
            ReactionAction::SendToAgent,
            EventPriority::Action,
            0,
            Some(1800),
            None,
            false,
        ),
        (
            "merge-conflicts",
            ReactionAction::SendToAgent,
            EventPriority::Action,
            0,
            Some(900),
            None,
            false,
        ),
        (
            "approved-and-green",
            ReactionAction::Notify,
            EventPriority::Action,
            0,
            None,
            None,
            false,
        ),
        (
            "agent-stuck",
            ReactionAction::Notify,
            EventPriority::Urgent,
            0,
            None,
            Some(600),
            false,
        ),
        (
            "agent-needs-input",
            ReactionAction::Notify,
            EventPriority::Urgent,
            0,
            None,
            None,
            false,
        ),
        (
            "agent-exited",
            ReactionAction::Notify,
            EventPriority::Urgent,
            0,
            None,
            None,
            false,
        ),
        (
            "all-complete",
            ReactionAction::Notify,
            EventPriority::Info,
            0,
            None,
            None,
            true,
        ),
    ];

    defs.iter()
        .map(
            |(name, action, priority, retries, escalate, threshold, summary)| {
                (
                    (*name).to_string(),
                    ReactionConfig {
                        enabled: true,
                        action: *action,
                        message: None,
                        priority: *priority,
                        retries: *retries,
                        escalate_after: escalate.map(Duration::from_secs),
                        threshold: threshold.map(Duration::from_secs),
                        include_summary: *summary,
                    },
                )
            },
        )
        .collect()
}

mod duration_millis {
    use std::time::Duration;

    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(u64::try_from(value.as_millis()).unwrap_or(u64::MAX))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reactions_has_all_nine() {
        let reactions = default_reactions();
        let expected = [
            "ci-failed",
            "changes-requested",
            "bugbot-comments",
            "merge-conflicts",
            "approved-and-green",
            "agent-stuck",
            "agent-needs-input",
            "agent-exited",
            "all-complete",
        ];
        for key in &expected {
            assert!(reactions.contains_key(*key), "missing reaction: {key}");
        }
        assert_eq!(reactions.len(), expected.len());
    }

    #[test]
    fn ci_failed_reaction_has_retries() {
        let reactions = default_reactions();
        let ci = &reactions["ci-failed"];
        assert_eq!(ci.retries, 2);
        assert!(ci.enabled);
    }

    #[test]
    fn default_config_yaml_roundtrip() {
        let config = OrchestratorConfig::default();
        let yaml = serde_yaml::to_string(&config).expect("serialize default config");
        let deserialized: OrchestratorConfig =
            serde_yaml::from_str(&yaml).expect("deserialize default config");

        assert_eq!(deserialized.port, config.port);
        assert_eq!(deserialized.terminal_port, config.terminal_port);
        assert_eq!(deserialized.ready_threshold, config.ready_threshold);
        assert_eq!(deserialized.projects.len(), config.projects.len());
        assert_eq!(deserialized.reactions.len(), config.reactions.len());
        assert_eq!(deserialized.defaults.runtime, config.defaults.runtime);
        assert_eq!(deserialized.defaults.agent, config.defaults.agent);
        assert_eq!(deserialized.defaults.workspace, config.defaults.workspace);
    }

    #[test]
    fn default_config_has_one_project() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.projects.len(), 1);
        assert_eq!(config.projects[0].name, "my-project");
        assert_eq!(config.projects[0].default_branch, "main");
    }

    #[test]
    fn default_config_has_default_reactions() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.reactions.len(), 9);
        assert!(config.reactions.contains_key("ci-failed"));
        assert!(config.reactions.contains_key("all-complete"));
    }
}

use std::collections::HashMap;
use std::fmt;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nats_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cors_origins: Vec<String>,
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
            database_url: None,
            nats_url: None,
            api_token: None,
            cors_origins: vec![],
        }
    }
}

const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";

impl OrchestratorConfig {
    pub fn resolve_database_url(&self) -> Option<String> {
        self.database_url
            .as_deref()
            .map(str::to_owned)
            .or_else(|| std::env::var("DATABASE_URL").ok())
    }

    pub fn resolve_nats_url(&self) -> String {
        self.nats_url
            .as_deref()
            .map(str::to_owned)
            .or_else(|| std::env::var("NATS_URL").ok())
            .unwrap_or_else(|| DEFAULT_NATS_URL.to_owned())
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_config: Option<SshConnectionConfig>,
}

impl ProjectConfig {
    pub fn is_remote(&self) -> bool {
        self.ssh_config.is_some()
    }
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
            ssh_config: None,
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SshAuthConfig {
    Key {
        path: PathBuf,
        #[serde(default, skip_serializing_if = "Option::is_none", skip_serializing)]
        passphrase: Option<String>,
    },
    Agent,
    Password {
        #[serde(skip_serializing)]
        password: String,
    },
}

impl fmt::Debug for SshAuthConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key { path, passphrase } => f
                .debug_struct("Key")
                .field("path", path)
                .field("passphrase", &passphrase.as_ref().map(|_| "[REDACTED]"))
                .finish(),
            Self::Agent => write!(f, "Agent"),
            Self::Password { .. } => f
                .debug_struct("Password")
                .field("password", &"[REDACTED]")
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshStrategyConfig {
    Tmux,
    Tmate,
    RemoteControl,
    Node,
}

impl Default for SshStrategyConfig {
    fn default() -> Self {
        Self::Tmux
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKeyPolicyConfig {
    Strict,
    AcceptNew,
    AcceptAll,
}

impl Default for HostKeyPolicyConfig {
    fn default() -> Self {
        Self::Strict
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnectionConfig {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub auth: SshAuthConfig,
    #[serde(default = "default_ssh_strategy")]
    pub strategy: SshStrategyConfig,
    #[serde(default = "default_ssh_timeout", with = "duration_secs")]
    pub connection_timeout: Duration,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_duration_secs"
    )]
    pub keepalive_interval: Option<Duration>,
    #[serde(default)]
    pub host_key_policy: HostKeyPolicyConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_config: Option<NodeConnectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnectionConfig {
    #[serde(default = "default_node_port")]
    pub port: u16,
    #[serde(default = "default_idle_timeout", with = "duration_secs")]
    pub idle_timeout: Duration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ennio_binary_path: Option<PathBuf>,
}

impl Default for NodeConnectionConfig {
    fn default() -> Self {
        Self {
            port: default_node_port(),
            idle_timeout: default_idle_timeout(),
            workspace_root: None,
            ennio_binary_path: None,
        }
    }
}

const fn default_node_port() -> u16 {
    9100
}

const fn default_idle_timeout() -> Duration {
    Duration::from_secs(3600)
}

const fn default_ssh_port() -> u16 {
    22
}

const fn default_ssh_strategy() -> SshStrategyConfig {
    SshStrategyConfig::Tmux
}

const fn default_ssh_timeout() -> Duration {
    Duration::from_secs(30)
}

mod duration_secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(value.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

mod option_duration_secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(d) => serializer.serialize_some(&d.as_secs()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<u64>::deserialize(deserializer)?;
        Ok(opt.map(Duration::from_secs))
    }
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
    use proptest::prelude::*;
    use rstest::rstest;

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

    #[test]
    fn project_is_remote_when_ssh_config_present() {
        let mut project = ProjectConfig::default();
        assert!(!project.is_remote());

        project.ssh_config = Some(SshConnectionConfig {
            host: "remote.example.com".to_string(),
            port: 22,
            username: "deploy".to_string(),
            auth: SshAuthConfig::Agent,
            strategy: SshStrategyConfig::Tmux,
            connection_timeout: Duration::from_secs(30),
            keepalive_interval: None,
            host_key_policy: HostKeyPolicyConfig::Strict,
            node_config: None,
        });
        assert!(project.is_remote());
    }

    #[test]
    fn ssh_password_debug_is_redacted() {
        let auth = SshAuthConfig::Password {
            password: "supersecret".to_string(),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("supersecret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn ssh_passphrase_debug_is_redacted() {
        let auth = SshAuthConfig::Key {
            path: PathBuf::from("/tmp/key"),
            passphrase: Some("mysecret".to_string()),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("mysecret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn ssh_connection_config_json_roundtrip() {
        let config = SshConnectionConfig {
            host: "host.example.com".to_string(),
            port: 2222,
            username: "deploy".to_string(),
            auth: SshAuthConfig::Agent,
            strategy: SshStrategyConfig::Tmux,
            connection_timeout: Duration::from_secs(60),
            keepalive_interval: Some(Duration::from_secs(15)),
            host_key_policy: HostKeyPolicyConfig::AcceptNew,
            node_config: None,
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let roundtripped: SshConnectionConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtripped.host, "host.example.com");
        assert_eq!(roundtripped.port, 2222);
        assert_eq!(roundtripped.strategy, SshStrategyConfig::Tmux);
        assert_eq!(roundtripped.connection_timeout, Duration::from_secs(60));
        assert_eq!(
            roundtripped.keepalive_interval,
            Some(Duration::from_secs(15))
        );
        assert_eq!(roundtripped.host_key_policy, HostKeyPolicyConfig::AcceptNew);
    }

    #[test]
    fn ssh_connection_config_defaults() {
        let json = r#"{
            "host": "example.com",
            "username": "user",
            "auth": {"type": "agent"}
        }"#;
        let config: SshConnectionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 22);
        assert_eq!(config.strategy, SshStrategyConfig::Tmux);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.host_key_policy, HostKeyPolicyConfig::Strict);
        assert!(config.keepalive_interval.is_none());
    }

    #[test]
    fn ssh_password_not_serialized() {
        let config = SshConnectionConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth: SshAuthConfig::Password {
                password: "secret123".to_string(),
            },
            strategy: SshStrategyConfig::Tmux,
            connection_timeout: Duration::from_secs(30),
            keepalive_interval: None,
            host_key_policy: HostKeyPolicyConfig::Strict,
            node_config: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("secret123"));
    }

    #[rstest]
    #[case("tmux", SshStrategyConfig::Tmux)]
    #[case("tmate", SshStrategyConfig::Tmate)]
    #[case("remote_control", SshStrategyConfig::RemoteControl)]
    #[case("node", SshStrategyConfig::Node)]
    fn ssh_strategy_config_deserializes(#[case] input: &str, #[case] expected: SshStrategyConfig) {
        let json = format!("\"{input}\"");
        let strategy: SshStrategyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, expected);
    }

    #[rstest]
    #[case("strict", HostKeyPolicyConfig::Strict)]
    #[case("accept_new", HostKeyPolicyConfig::AcceptNew)]
    #[case("accept_all", HostKeyPolicyConfig::AcceptAll)]
    fn host_key_policy_config_deserializes(
        #[case] input: &str,
        #[case] expected: HostKeyPolicyConfig,
    ) {
        let json = format!("\"{input}\"");
        let policy: HostKeyPolicyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, expected);
    }

    proptest! {
        #[test]
        fn ssh_connection_config_roundtrip(
            host in "[a-z]{3,10}\\.[a-z]{2,5}",
            port in 1u16..=65535,
            username in "[a-z]{3,12}",
            timeout_secs in 1u64..=300,
        ) {
            let config = SshConnectionConfig {
                host,
                port,
                username,
                auth: SshAuthConfig::Agent,
                strategy: SshStrategyConfig::Tmux,
                connection_timeout: Duration::from_secs(timeout_secs),
                keepalive_interval: None,
                host_key_policy: HostKeyPolicyConfig::Strict,
                node_config: None,
            };
            let json = serde_json::to_string(&config).unwrap();
            let roundtripped: SshConnectionConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(roundtripped.port, config.port);
            prop_assert_eq!(roundtripped.host, config.host);
            prop_assert_eq!(roundtripped.username, config.username);
            prop_assert_eq!(roundtripped.connection_timeout, config.connection_timeout);
        }
    }

    #[test]
    fn project_with_ssh_config_yaml_roundtrip() {
        let project = ProjectConfig {
            ssh_config: Some(SshConnectionConfig {
                host: "remote.example.com".to_string(),
                port: 22,
                username: "deploy".to_string(),
                auth: SshAuthConfig::Agent,
                strategy: SshStrategyConfig::Tmux,
                connection_timeout: Duration::from_secs(30),
                keepalive_interval: None,
                host_key_policy: HostKeyPolicyConfig::Strict,
                node_config: None,
            }),
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&project).expect("serialize");
        let roundtripped: ProjectConfig = serde_yaml::from_str(&yaml).expect("deserialize");
        assert!(roundtripped.is_remote());
        let ssh = roundtripped.ssh_config.unwrap();
        assert_eq!(ssh.host, "remote.example.com");
        assert_eq!(ssh.username, "deploy");
    }

    #[test]
    fn default_project_has_no_ssh_config() {
        let project = ProjectConfig::default();
        assert!(!project.is_remote());
        assert!(project.ssh_config.is_none());
    }

    #[test]
    fn node_strategy_deserializes() {
        let json = r#"{
            "host": "example.com",
            "username": "user",
            "auth": {"type": "agent"},
            "strategy": "node",
            "node_config": {
                "port": 9200,
                "idle_timeout": 1800
            }
        }"#;
        let config: SshConnectionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.strategy, SshStrategyConfig::Node);
        let node = config.node_config.unwrap();
        assert_eq!(node.port, 9200);
        assert_eq!(node.idle_timeout, Duration::from_secs(1800));
    }

    #[test]
    fn node_connection_config_defaults() {
        let config = NodeConnectionConfig::default();
        assert_eq!(config.port, 9100);
        assert_eq!(config.idle_timeout, Duration::from_secs(3600));
        assert!(config.workspace_root.is_none());
        assert!(config.ennio_binary_path.is_none());
    }

    proptest! {
        #[test]
        fn node_connection_config_roundtrip(
            port in 1024u16..=65535,
            idle_secs in 60u64..=7200,
        ) {
            let config = NodeConnectionConfig {
                port,
                idle_timeout: Duration::from_secs(idle_secs),
                workspace_root: None,
                ennio_binary_path: None,
            };
            let json = serde_json::to_string(&config).unwrap();
            let roundtripped: NodeConnectionConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(roundtripped.port, config.port);
            prop_assert_eq!(roundtripped.idle_timeout, config.idle_timeout);
        }
    }

    #[test]
    fn ssh_connection_config_without_node_config() {
        let json = r#"{
            "host": "example.com",
            "username": "user",
            "auth": {"type": "agent"}
        }"#;
        let config: SshConnectionConfig = serde_json::from_str(json).unwrap();
        assert!(config.node_config.is_none());
    }

    #[test]
    fn default_config_new_fields_are_none_or_empty() {
        let config = OrchestratorConfig::default();
        assert!(config.database_url.is_none());
        assert!(config.nats_url.is_none());
        assert!(config.api_token.is_none());
        assert!(config.cors_origins.is_empty());
    }

    #[rstest]
    #[case(Some("postgres://localhost/ennio"), None, "postgres://localhost/ennio")]
    #[case(None, Some("postgres://env/ennio"), "postgres://env/ennio")]
    fn resolve_database_url_precedence(
        #[case] config_val: Option<&str>,
        #[case] env_val: Option<&str>,
        #[case] expected: &str,
    ) {
        let config = OrchestratorConfig {
            database_url: config_val.map(str::to_owned),
            ..Default::default()
        };

        unsafe {
            if let Some(val) = env_val {
                std::env::set_var("DATABASE_URL", val);
            } else {
                std::env::remove_var("DATABASE_URL");
            }
        }

        let result = config.resolve_database_url();
        assert_eq!(result.as_deref(), Some(expected));

        unsafe { std::env::remove_var("DATABASE_URL") };
    }

    #[test]
    fn resolve_database_url_returns_none_when_both_absent() {
        let config = OrchestratorConfig::default();
        unsafe { std::env::remove_var("DATABASE_URL") };
        assert!(config.resolve_database_url().is_none());
    }

    #[test]
    fn resolve_nats_url_uses_default() {
        let config = OrchestratorConfig::default();
        unsafe { std::env::remove_var("NATS_URL") };
        assert_eq!(config.resolve_nats_url(), "nats://127.0.0.1:4222");
    }

    #[test]
    fn resolve_nats_url_config_takes_precedence() {
        let config = OrchestratorConfig {
            nats_url: Some("nats://custom:4222".to_owned()),
            ..Default::default()
        };
        unsafe { std::env::set_var("NATS_URL", "nats://env:4222") };

        assert_eq!(config.resolve_nats_url(), "nats://custom:4222");

        unsafe { std::env::remove_var("NATS_URL") };
    }

    #[test]
    fn config_with_new_fields_yaml_roundtrip() {
        let config = OrchestratorConfig {
            database_url: Some("postgres://localhost/ennio".to_owned()),
            nats_url: Some("nats://localhost:4222".to_owned()),
            api_token: Some("secret-token".to_owned()),
            cors_origins: vec!["http://localhost:3000".to_owned()],
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&config).expect("serialize");
        let deserialized: OrchestratorConfig = serde_yaml::from_str(&yaml).expect("deserialize");

        assert_eq!(
            deserialized.database_url.as_deref(),
            Some("postgres://localhost/ennio")
        );
        assert_eq!(
            deserialized.nats_url.as_deref(),
            Some("nats://localhost:4222")
        );
        assert_eq!(deserialized.api_token.as_deref(), Some("secret-token"));
        assert_eq!(deserialized.cors_origins, vec!["http://localhost:3000"]);
    }
}

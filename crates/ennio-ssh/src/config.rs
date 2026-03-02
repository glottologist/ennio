use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    #[serde(default = "default_connection_timeout", with = "duration_secs")]
    pub connection_timeout: Duration,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "option_duration_secs"
    )]
    pub keepalive_interval: Option<Duration>,
    #[serde(default)]
    pub host_key_policy: HostKeyPolicy,
}

const fn default_port() -> u16 {
    22
}

const fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SshAuth {
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

impl std::fmt::Debug for SshAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
pub enum HostKeyPolicy {
    Strict,
    AcceptNew,
    AcceptAll,
}

impl Default for HostKeyPolicy {
    fn default() -> Self {
        Self::Strict
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshStrategy {
    Tmux,
    Tmate,
    RemoteControl,
    Node,
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

impl From<ennio_core::config::SshAuthConfig> for SshAuth {
    fn from(auth: ennio_core::config::SshAuthConfig) -> Self {
        match auth {
            ennio_core::config::SshAuthConfig::Key { path, passphrase } => {
                Self::Key { path, passphrase }
            }
            ennio_core::config::SshAuthConfig::Agent => Self::Agent,
            ennio_core::config::SshAuthConfig::Password { password } => Self::Password { password },
        }
    }
}

impl From<ennio_core::config::SshStrategyConfig> for SshStrategy {
    fn from(strategy: ennio_core::config::SshStrategyConfig) -> Self {
        match strategy {
            ennio_core::config::SshStrategyConfig::Tmux => Self::Tmux,
            ennio_core::config::SshStrategyConfig::Tmate => Self::Tmate,
            ennio_core::config::SshStrategyConfig::RemoteControl => Self::RemoteControl,
            ennio_core::config::SshStrategyConfig::Node => Self::Node,
        }
    }
}

impl From<ennio_core::config::HostKeyPolicyConfig> for HostKeyPolicy {
    fn from(policy: ennio_core::config::HostKeyPolicyConfig) -> Self {
        match policy {
            ennio_core::config::HostKeyPolicyConfig::Strict => Self::Strict,
            ennio_core::config::HostKeyPolicyConfig::AcceptNew => Self::AcceptNew,
            ennio_core::config::HostKeyPolicyConfig::AcceptAll => Self::AcceptAll,
        }
    }
}

impl From<ennio_core::config::SshConnectionConfig> for SshConfig {
    fn from(config: ennio_core::config::SshConnectionConfig) -> Self {
        Self {
            host: config.host,
            port: config.port,
            username: config.username,
            auth: config.auth.into(),
            connection_timeout: config.connection_timeout,
            keepalive_interval: config.keepalive_interval,
            host_key_policy: config.host_key_policy.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("tmux", SshStrategy::Tmux)]
    #[case("tmate", SshStrategy::Tmate)]
    #[case("remote_control", SshStrategy::RemoteControl)]
    #[case("node", SshStrategy::Node)]
    fn strategy_deserializes(#[case] input: &str, #[case] expected: SshStrategy) {
        let json = format!("\"{input}\"");
        let strategy: SshStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, expected);
    }

    proptest! {
        #[test]
        fn ssh_config_roundtrip_with_agent_auth(
            host in "[a-z]{3,10}\\.[a-z]{2,5}",
            port in 1u16..=65535,
            username in "[a-z]{3,12}",
            timeout_secs in 1u64..=300,
        ) {
            let config = SshConfig {
                host,
                port,
                username,
                auth: SshAuth::Agent,
                connection_timeout: Duration::from_secs(timeout_secs),
                keepalive_interval: None,
                host_key_policy: HostKeyPolicy::Strict,
            };
            let json = serde_json::to_string(&config).unwrap();
            let roundtripped: SshConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(roundtripped.port, config.port);
            prop_assert_eq!(roundtripped.host, config.host);
            prop_assert_eq!(roundtripped.username, config.username);
        }
    }

    #[test]
    fn password_not_serialized() {
        let config = SshConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth: SshAuth::Password {
                password: "secret123".to_string(),
            },
            connection_timeout: Duration::from_secs(30),
            keepalive_interval: None,
            host_key_policy: HostKeyPolicy::Strict,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("secret123"));
    }

    #[test]
    fn default_port_is_22() {
        let json = r#"{
            "host": "example.com",
            "username": "user",
            "auth": {"type": "agent"},
            "connection_timeout": 30
        }"#;
        let config: SshConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.port, 22);
    }

    #[test]
    fn key_auth_with_passphrase_roundtrips() {
        let config = SshConfig {
            host: "host.example.com".to_string(),
            port: 2222,
            username: "deploy".to_string(),
            auth: SshAuth::Key {
                path: PathBuf::from("/home/user/.ssh/id_ed25519"),
                passphrase: Some("secret".to_string()),
            },
            connection_timeout: Duration::from_secs(10),
            keepalive_interval: Some(Duration::from_secs(60)),
            host_key_policy: HostKeyPolicy::Strict,
        };
        let json = serde_json::to_string(&config).unwrap();
        let roundtripped: SshConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.port, 2222);
        assert_eq!(
            roundtripped.keepalive_interval,
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn password_debug_is_redacted() {
        let auth = SshAuth::Password {
            password: "supersecret".to_string(),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("supersecret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn passphrase_debug_is_redacted() {
        let auth = SshAuth::Key {
            path: PathBuf::from("/tmp/key"),
            passphrase: Some("mysecret".to_string()),
        };
        let debug = format!("{auth:?}");
        assert!(!debug.contains("mysecret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn default_host_key_policy_is_strict() {
        let json = r#"{
            "host": "example.com",
            "username": "user",
            "auth": {"type": "agent"},
            "connection_timeout": 30
        }"#;
        let config: SshConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.host_key_policy, HostKeyPolicy::Strict);
    }

    #[rstest]
    #[case(ennio_core::config::SshStrategyConfig::Tmux, SshStrategy::Tmux)]
    #[case(ennio_core::config::SshStrategyConfig::Tmate, SshStrategy::Tmate)]
    #[case(
        ennio_core::config::SshStrategyConfig::RemoteControl,
        SshStrategy::RemoteControl
    )]
    #[case(ennio_core::config::SshStrategyConfig::Node, SshStrategy::Node)]
    fn strategy_config_converts(
        #[case] input: ennio_core::config::SshStrategyConfig,
        #[case] expected: SshStrategy,
    ) {
        let result: SshStrategy = input.into();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(ennio_core::config::HostKeyPolicyConfig::Strict, HostKeyPolicy::Strict)]
    #[case(
        ennio_core::config::HostKeyPolicyConfig::AcceptNew,
        HostKeyPolicy::AcceptNew
    )]
    #[case(
        ennio_core::config::HostKeyPolicyConfig::AcceptAll,
        HostKeyPolicy::AcceptAll
    )]
    fn host_key_policy_config_converts(
        #[case] input: ennio_core::config::HostKeyPolicyConfig,
        #[case] expected: HostKeyPolicy,
    ) {
        let result: HostKeyPolicy = input.into();
        assert_eq!(result, expected);
    }

    #[test]
    fn ssh_auth_agent_converts() {
        let input = ennio_core::config::SshAuthConfig::Agent;
        let result: SshAuth = input.into();
        assert!(matches!(result, SshAuth::Agent));
    }

    #[test]
    fn ssh_auth_key_converts() {
        let input = ennio_core::config::SshAuthConfig::Key {
            path: PathBuf::from("/home/user/.ssh/id_ed25519"),
            passphrase: Some("secret".to_string()),
        };
        let result: SshAuth = input.into();
        match result {
            SshAuth::Key { path, passphrase } => {
                assert_eq!(path, PathBuf::from("/home/user/.ssh/id_ed25519"));
                assert_eq!(passphrase, Some("secret".to_string()));
            }
            _ => panic!("expected Key variant"),
        }
    }

    #[test]
    fn ssh_connection_config_converts() {
        let input = ennio_core::config::SshConnectionConfig {
            host: "remote.example.com".to_string(),
            port: 2222,
            username: "deploy".to_string(),
            auth: ennio_core::config::SshAuthConfig::Agent,
            strategy: ennio_core::config::SshStrategyConfig::Tmate,
            connection_timeout: Duration::from_secs(60),
            keepalive_interval: Some(Duration::from_secs(15)),
            host_key_policy: ennio_core::config::HostKeyPolicyConfig::AcceptNew,
            node_config: None,
        };
        let result: SshConfig = input.into();
        assert_eq!(result.host, "remote.example.com");
        assert_eq!(result.port, 2222);
        assert_eq!(result.username, "deploy");
        assert!(matches!(result.auth, SshAuth::Agent));
        assert_eq!(result.connection_timeout, Duration::from_secs(60));
        assert_eq!(result.keepalive_interval, Some(Duration::from_secs(15)));
        assert_eq!(result.host_key_policy, HostKeyPolicy::AcceptNew);
    }
}

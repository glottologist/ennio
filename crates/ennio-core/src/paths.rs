use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::error::EnnioError;

/// Compute a 12-char hex hash from a config directory path.
/// Matches TS: sha256(configDir).slice(0, 12)
pub fn config_hash(config_dir: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(config_dir.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..6])
}

pub fn base_data_dir() -> Result<PathBuf, EnnioError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| EnnioError::Config {
            message: "HOME environment variable not set".to_owned(),
        })?;
    Ok(home.join(".ennio"))
}

pub fn data_dir(hash: &str, project_id: &str) -> Result<PathBuf, EnnioError> {
    Ok(base_data_dir()?.join(format!("{hash}-{project_id}")))
}

pub fn sessions_dir(hash: &str, project_id: &str) -> Result<PathBuf, EnnioError> {
    Ok(data_dir(hash, project_id)?.join("sessions"))
}

pub fn worktrees_dir(hash: &str, project_id: &str) -> Result<PathBuf, EnnioError> {
    Ok(data_dir(hash, project_id)?.join("worktrees"))
}

pub fn archive_dir(hash: &str, project_id: &str) -> Result<PathBuf, EnnioError> {
    Ok(data_dir(hash, project_id)?.join("archive"))
}

/// Generate a session prefix from a project name.
/// CamelCase → uppercase letters (e.g. "AgentOrchestrator" → "ao")
/// kebab-case → first letter of each segment (e.g. "my-project" → "mp")
/// single word → first 3 chars (e.g. "integrator" → "int")
pub fn session_prefix_from_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    if name.contains('-') {
        return name
            .split('-')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.chars().next())
            .collect::<String>()
            .to_lowercase();
    }

    let uppers: String = name.chars().filter(|c| c.is_ascii_uppercase()).collect();
    if uppers.len() >= 2 {
        return uppers.to_lowercase();
    }

    let prefix: String = name.chars().take(3).collect();
    prefix.to_lowercase()
}

pub fn tmux_name(hash: &str, prefix: &str, num: u32) -> String {
    format!("{hash}-{prefix}-{num}")
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    proptest! {
        #[test]
        fn config_hash_deterministic(s in "\\PC{1,200}") {
            let a = config_hash(&s);
            let b = config_hash(&s);
            assert_eq!(a, b);
        }

        #[test]
        fn config_hash_is_12_hex_chars(s in "\\PC{1,200}") {
            let h = config_hash(&s);
            assert_eq!(h.len(), 12);
            assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn data_dir_under_base(hash in "[a-f0-9]{12}", project in "[a-z]{1,10}") {
            let d = data_dir(&hash, &project).unwrap();
            assert!(d.starts_with(base_data_dir().unwrap()));
        }
    }

    #[rstest]
    #[case("AgentOrchestrator", "ao")]
    #[case("ClaudeCode", "cc")]
    #[case("MyBigProject", "mbp")]
    fn camel_case_prefix(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(session_prefix_from_name(input), expected);
    }

    #[rstest]
    #[case("my-project", "mp")]
    #[case("agent-orchestrator", "ao")]
    #[case("some-big-thing", "sbt")]
    fn kebab_case_prefix(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(session_prefix_from_name(input), expected);
    }

    #[rstest]
    #[case("integrator", "int")]
    #[case("backend", "bac")]
    #[case("api", "api")]
    fn single_word_prefix(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(session_prefix_from_name(input), expected);
    }

    #[test]
    fn empty_name_returns_empty() {
        assert_eq!(session_prefix_from_name(""), "");
    }

    proptest! {
        #[test]
        fn tmux_name_format(
            hash in "[a-f0-9]{12}",
            prefix in "[a-z]{1,5}",
            num in 0..1000u32,
        ) {
            let name = tmux_name(&hash, &prefix, num);
            prop_assert_eq!(name, format!("{hash}-{prefix}-{num}"));
        }
    }
}

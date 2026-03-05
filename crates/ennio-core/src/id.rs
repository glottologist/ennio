use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::EnnioError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

fn is_valid_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn validate_id(value: &str, kind: &str) -> Result<(), EnnioError> {
    if value.is_empty() {
        return Err(EnnioError::InvalidId {
            value: value.to_string(),
            reason: format!("{kind} cannot be empty"),
        });
    }
    if let Some(bad) = value.chars().find(|c| !is_valid_id_char(*c)) {
        return Err(EnnioError::InvalidId {
            value: value.to_string(),
            reason: format!("{kind} contains invalid character: '{bad}'"),
        });
    }
    Ok(())
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, EnnioError> {
        let s = value.into();
        validate_id(&s, "SessionId")?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ProjectId {
    pub fn new(value: impl Into<String>) -> Result<Self, EnnioError> {
        let s = value.into();
        validate_id(&s, "ProjectId")?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl EventId {
    pub fn new(value: impl Into<String>) -> Result<Self, EnnioError> {
        let s = value.into();
        validate_id(&s, "EventId")?;
        Ok(Self(s))
    }

    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn valid_session_ids_accepted(s in "[a-zA-Z0-9_-]{1,64}") {
            let id = SessionId::new(s.clone()).unwrap();
            assert_eq!(id.as_str(), s);
        }

        #[test]
        fn display_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
            let id = SessionId::new(s.clone()).unwrap();
            assert_eq!(id.to_string(), s);
        }

        #[test]
        fn invalid_chars_rejected(s in ".*[^a-zA-Z0-9_-].*") {
            prop_assume!(!s.is_empty());
            assert!(SessionId::new(s).is_err());
        }
    }

    #[test]
    fn empty_string_rejected() {
        assert!(SessionId::new("").is_err());
    }

    #[test]
    fn event_id_random_is_unique() {
        let a = EventId::random();
        let b = EventId::random();
        assert_ne!(a, b);
    }
}

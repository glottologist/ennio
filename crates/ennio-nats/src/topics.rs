use crate::error::NatsError;
use ennio_core::event::EventType;

const PREFIX: &str = "ennio";

const INVALID_SEGMENT_CHARS: &[char] = &[' ', '.', '*', '>', '\t', '\n', '\r', '\0'];

fn validate_segment(segment: &str) -> Result<(), NatsError> {
    if segment.is_empty() {
        return Err(NatsError::InvalidTopic("empty segment".to_string()));
    }
    if let Some(ch) = segment.chars().find(|c| INVALID_SEGMENT_CHARS.contains(c)) {
        return Err(NatsError::InvalidTopic(format!(
            "segment contains invalid character {ch:?}: '{segment}'"
        )));
    }
    Ok(())
}

pub fn session_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "sessions", project_id, action)
}

pub fn pr_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "pr", project_id, action)
}

pub fn ci_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "ci", project_id, action)
}

pub fn review_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "review", project_id, action)
}

pub fn merge_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "merge", project_id, action)
}

pub fn reactions_topic(project_id: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "reactions", project_id, action)
}

pub fn lifecycle_topic(action: &str) -> Result<String, NatsError> {
    validate_segment(action)?;
    Ok([PREFIX, "lifecycle", action].join("."))
}

pub fn commands_topic(command: &str) -> Result<String, NatsError> {
    validate_segment(command)?;
    Ok([PREFIX, "commands", command].join("."))
}

pub fn metrics_topic(action: &str) -> Result<String, NatsError> {
    validate_segment(action)?;
    Ok([PREFIX, "metrics", action].join("."))
}

pub fn dashboard_topic(action: &str) -> Result<String, NatsError> {
    validate_segment(action)?;
    Ok([PREFIX, "dashboard", action].join("."))
}

pub fn node_topic(host: &str, action: &str) -> Result<String, NatsError> {
    build_topic(PREFIX, "node", host, action)
}

pub fn node_subscribe_pattern(host: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "node", host)
}

pub fn session_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "sessions", project_id)
}

pub fn pr_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "pr", project_id)
}

pub fn ci_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "ci", project_id)
}

pub fn review_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "review", project_id)
}

pub fn merge_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "merge", project_id)
}

pub fn reactions_subscribe_pattern(project_id: &str) -> Result<String, NatsError> {
    subscribe_pattern(PREFIX, "reactions", project_id)
}

pub fn topic_for_event_type(event_type: EventType, project_id: &str) -> Result<String, NatsError> {
    match event_type {
        EventType::SessionSpawned => session_topic(project_id, "spawned"),
        EventType::SessionWorking => session_topic(project_id, "working"),
        EventType::SessionExited => session_topic(project_id, "exited"),
        EventType::SessionKilled => session_topic(project_id, "killed"),
        EventType::SessionRestored => session_topic(project_id, "restored"),
        EventType::SessionCleaned => session_topic(project_id, "cleaned"),
        EventType::StatusChanged => session_topic(project_id, "status_changed"),
        EventType::ActivityChanged => session_topic(project_id, "activity_changed"),
        EventType::PrCreated => pr_topic(project_id, "created"),
        EventType::PrUpdated => pr_topic(project_id, "updated"),
        EventType::PrMerged => pr_topic(project_id, "merged"),
        EventType::PrClosed => pr_topic(project_id, "closed"),
        EventType::CiPassing => ci_topic(project_id, "passing"),
        EventType::CiFailing => ci_topic(project_id, "failing"),
        EventType::CiFixSent => ci_topic(project_id, "fix_sent"),
        EventType::CiFixFailed => ci_topic(project_id, "fix_failed"),
        EventType::ReviewPending => review_topic(project_id, "pending"),
        EventType::ReviewApproved => review_topic(project_id, "approved"),
        EventType::ReviewChangesRequested => review_topic(project_id, "changes_requested"),
        EventType::ReviewCommentsSent => review_topic(project_id, "comments_sent"),
        EventType::MergeReady => merge_topic(project_id, "ready"),
        EventType::MergeConflicts => merge_topic(project_id, "conflicts"),
        EventType::MergeCompleted => merge_topic(project_id, "completed"),
        EventType::ReactionTriggered => reactions_topic(project_id, "triggered"),
        EventType::ReactionEscalated => reactions_topic(project_id, "escalated"),
        EventType::AllComplete => lifecycle_topic("all_complete"),
        EventType::NodeConnected => node_topic(project_id, "connected"),
        EventType::NodeDisconnected => node_topic(project_id, "disconnected"),
        EventType::NodeLaunched => node_topic(project_id, "launched"),
        EventType::NodeHealthCheck => node_topic(project_id, "health_check"),
    }
}

fn build_topic(
    prefix: &str,
    domain: &str,
    project_id: &str,
    action: &str,
) -> Result<String, NatsError> {
    validate_segment(project_id)?;
    validate_segment(action)?;
    Ok([prefix, domain, project_id, action].join("."))
}

fn subscribe_pattern(prefix: &str, domain: &str, project_id: &str) -> Result<String, NatsError> {
    validate_segment(project_id)?;
    Ok([prefix, domain, project_id, "*"].join("."))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    proptest! {
        #[test]
        fn session_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = session_topic(&project_id, "spawned").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.sessions."));
            prop_assert!(topic.ends_with(".spawned"));
        }

        #[test]
        fn pr_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = pr_topic(&project_id, "created").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.pr."));
            prop_assert!(topic.ends_with(".created"));
        }

        #[test]
        fn ci_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = ci_topic(&project_id, "passing").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.ci."));
            prop_assert!(topic.ends_with(".passing"));
        }

        #[test]
        fn review_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = review_topic(&project_id, "pending").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.review."));
            prop_assert!(topic.ends_with(".pending"));
        }

        #[test]
        fn merge_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = merge_topic(&project_id, "ready").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.merge."));
            prop_assert!(topic.ends_with(".ready"));
        }

        #[test]
        fn reactions_topic_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let topic = reactions_topic(&project_id, "triggered").unwrap();
            prop_assert!(topic.contains(&project_id));
            prop_assert!(topic.starts_with("ennio.reactions."));
            prop_assert!(topic.ends_with(".triggered"));
        }

        #[test]
        fn subscribe_pattern_ends_with_wildcard(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let pattern = session_subscribe_pattern(&project_id).unwrap();
            prop_assert!(pattern.contains(&project_id));
            prop_assert!(pattern.ends_with(".*"));
            prop_assert!(pattern.starts_with("ennio.sessions."));
        }

        #[test]
        fn topic_has_four_segments(
            project_id in "[a-zA-Z0-9_-]{1,64}",
            action in "[a-z_]{1,32}"
        ) {
            let topic = session_topic(&project_id, &action).unwrap();
            let segments: Vec<&str> = topic.split('.').collect();
            prop_assert_eq!(segments.len(), 4);
            prop_assert_eq!(segments[0], "ennio");
            prop_assert_eq!(segments[1], "sessions");
            prop_assert_eq!(segments[2], project_id.as_str());
            prop_assert_eq!(segments[3], action.as_str());
        }

        #[test]
        fn event_type_mapping_always_contains_project_id(project_id in "[a-zA-Z0-9_-]{1,64}") {
            let event_types = [
                EventType::SessionSpawned,
                EventType::SessionWorking,
                EventType::SessionExited,
                EventType::SessionKilled,
                EventType::SessionRestored,
                EventType::SessionCleaned,
                EventType::StatusChanged,
                EventType::ActivityChanged,
                EventType::PrCreated,
                EventType::PrUpdated,
                EventType::PrMerged,
                EventType::PrClosed,
                EventType::CiPassing,
                EventType::CiFailing,
                EventType::CiFixSent,
                EventType::CiFixFailed,
                EventType::ReviewPending,
                EventType::ReviewApproved,
                EventType::ReviewChangesRequested,
                EventType::ReviewCommentsSent,
                EventType::MergeReady,
                EventType::MergeConflicts,
                EventType::MergeCompleted,
                EventType::ReactionTriggered,
                EventType::ReactionEscalated,
                EventType::NodeConnected,
                EventType::NodeDisconnected,
                EventType::NodeLaunched,
                EventType::NodeHealthCheck,
            ];

            for event_type in event_types {
                let topic = topic_for_event_type(event_type, &project_id).unwrap();
                prop_assert!(
                    topic.contains(&project_id),
                    "topic for {:?} does not contain project_id: {}",
                    event_type,
                    topic
                );
                prop_assert!(topic.starts_with("ennio."));
            }
        }

        #[test]
        fn validate_segment_rejects_empty(s in "[ .]{0,10}") {
            if s.is_empty() || s.contains(' ') || s.contains('.') {
                prop_assert!(validate_segment(&s).is_err());
            }
        }

        #[test]
        fn validate_segment_accepts_valid(s in "[a-zA-Z0-9_-]{1,64}") {
            prop_assert!(validate_segment(&s).is_ok());
        }
    }

    #[rstest]
    #[case("test*", "NATS single-level wildcard")]
    #[case("test>", "NATS multi-level wildcard")]
    #[case("*", "bare single-level wildcard")]
    #[case(">", "bare multi-level wildcard")]
    #[case("foo*bar", "embedded single-level wildcard")]
    #[case("foo>bar", "embedded multi-level wildcard")]
    #[case("test\tvalue", "tab character")]
    #[case("test\nvalue", "newline character")]
    #[case("test\rvalue", "carriage return")]
    #[case("test\0value", "null byte")]
    fn validate_segment_rejects_nats_special_chars(#[case] input: &str, #[case] label: &str) {
        let result = validate_segment(input);
        assert!(result.is_err(), "expected rejection for {label}: {input:?}");
    }

    #[rstest]
    #[case("test*project", "wildcard in project_id")]
    #[case("test>project", "multi-level wildcard in project_id")]
    fn session_topic_rejects_wildcard_project_id(#[case] project_id: &str, #[case] label: &str) {
        let result = session_topic(project_id, "spawned");
        assert!(
            result.is_err(),
            "expected rejection for {label}: {project_id:?}"
        );
    }

    #[rstest]
    #[case("test*action", "wildcard in action")]
    #[case("test>action", "multi-level wildcard in action")]
    fn session_topic_rejects_wildcard_action(#[case] action: &str, #[case] label: &str) {
        let result = session_topic("valid-project", action);
        assert!(
            result.is_err(),
            "expected rejection for {label}: {action:?}"
        );
    }

    #[rstest]
    #[case("lifecycle", "poll_started", "ennio.lifecycle.poll_started")]
    #[case("commands", "spawn", "ennio.commands.spawn")]
    #[case("metrics", "cost_recorded", "ennio.metrics.cost_recorded")]
    #[case("dashboard", "sessions_updated", "ennio.dashboard.sessions_updated")]
    fn three_segment_topic_format(
        #[case] domain: &str,
        #[case] action: &str,
        #[case] expected: &str,
    ) {
        let topic = match domain {
            "lifecycle" => lifecycle_topic(action),
            "commands" => commands_topic(action),
            "metrics" => metrics_topic(action),
            "dashboard" => dashboard_topic(action),
            other => panic!("unexpected domain in test data: {other}"),
        };
        assert_eq!(topic.unwrap(), expected);
    }

    #[test]
    fn all_complete_maps_to_lifecycle() {
        let topic = topic_for_event_type(EventType::AllComplete, "any-project").unwrap();
        assert_eq!(topic, "ennio.lifecycle.all_complete");
    }

    #[test]
    fn node_topic_has_four_segments() {
        let topic = node_topic("remote-host", "connected").unwrap();
        assert_eq!(topic, "ennio.node.remote-host.connected");
    }

    #[test]
    fn node_subscribe_pattern_ends_with_wildcard() {
        let pattern = node_subscribe_pattern("remote-host").unwrap();
        assert_eq!(pattern, "ennio.node.remote-host.*");
    }

    #[rstest]
    #[case(EventType::NodeConnected, "my-host", "ennio.node.my-host.connected")]
    #[case(
        EventType::NodeDisconnected,
        "my-host",
        "ennio.node.my-host.disconnected"
    )]
    #[case(EventType::NodeLaunched, "my-host", "ennio.node.my-host.launched")]
    #[case(
        EventType::NodeHealthCheck,
        "my-host",
        "ennio.node.my-host.health_check"
    )]
    fn node_event_type_mapping(
        #[case] event_type: EventType,
        #[case] host: &str,
        #[case] expected: &str,
    ) {
        let topic = topic_for_event_type(event_type, host).unwrap();
        assert_eq!(topic, expected);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PromptLayer {
    System,
    Project,
    Issue,
    AgentRules,
    User,
}

#[derive(Debug, Clone)]
struct LayerEntry {
    layer: PromptLayer,
    content: String,
}

#[derive(Debug, Clone, Default)]
pub struct PromptBuilder {
    layers: Vec<LayerEntry>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_layer(&mut self, layer: PromptLayer, content: impl Into<String>) -> &mut Self {
        self.layers.push(LayerEntry {
            layer,
            content: content.into(),
        });
        self
    }

    pub fn compose(&self) -> String {
        let mut sorted: Vec<&LayerEntry> = self.layers.iter().collect();
        sorted.sort_by_key(|e| e.layer);

        sorted
            .iter()
            .map(|e| e.content.as_str())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub fn compose_prompt(
    system: Option<&str>,
    project: Option<&str>,
    issue: Option<&str>,
    agent_rules: &[&str],
    user: Option<&str>,
) -> String {
    let mut builder = PromptBuilder::new();

    if let Some(s) = system {
        builder.add_layer(PromptLayer::System, s);
    }
    if let Some(p) = project {
        builder.add_layer(PromptLayer::Project, p);
    }
    if let Some(i) = issue {
        builder.add_layer(PromptLayer::Issue, i);
    }
    for rule in agent_rules {
        builder.add_layer(PromptLayer::AgentRules, *rule);
    }
    if let Some(u) = user {
        builder.add_layer(PromptLayer::User, u);
    }

    builder.compose()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn compose_preserves_layer_order(
            system in "[a-z]{1,20}",
            project in "[a-z]{1,20}",
            user in "[a-z]{1,20}",
        ) {
            let sys_tagged = format!("SYS:{system}");
            let proj_tagged = format!("PRJ:{project}");
            let usr_tagged = format!("USR:{user}");
            let result = compose_prompt(
                Some(&sys_tagged),
                Some(&proj_tagged),
                None,
                &[],
                Some(&usr_tagged),
            );
            let sys_pos = result.find(&sys_tagged).unwrap();
            let proj_pos = result.find(&proj_tagged).unwrap();
            let user_pos = result.find(&usr_tagged).unwrap();
            prop_assert!(sys_pos < proj_pos);
            prop_assert!(proj_pos < user_pos);
        }

        #[test]
        fn empty_layers_produce_no_garbage(
            content in "[a-z]{1,20}",
        ) {
            let result = compose_prompt(Some(&content), None, None, &[], None);
            assert_eq!(result, content);
        }
    }

    #[test]
    fn all_layers_composed() {
        let result = compose_prompt(
            Some("sys"),
            Some("proj"),
            Some("issue"),
            &["rule1", "rule2"],
            Some("user"),
        );
        assert!(result.contains("sys"));
        assert!(result.contains("proj"));
        assert!(result.contains("issue"));
        assert!(result.contains("rule1"));
        assert!(result.contains("rule2"));
        assert!(result.contains("user"));
    }
}

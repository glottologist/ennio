use std::collections::HashMap;
use std::sync::Arc;

use ennio_core::agent::Agent;
use ennio_core::error::EnnioError;
use ennio_core::notifier::Notifier;
use ennio_core::plugin::{PluginManifest, PluginSlot};
use ennio_core::runtime::Runtime;
use ennio_core::scm::Scm;
use ennio_core::terminal::Terminal;
use ennio_core::tracker::Tracker;
use ennio_core::workspace::Workspace;

pub struct PluginRegistry {
    runtimes: HashMap<String, Arc<dyn Runtime>>,
    agents: HashMap<String, Arc<dyn Agent>>,
    workspaces: HashMap<String, Arc<dyn Workspace>>,
    trackers: HashMap<String, Arc<dyn Tracker>>,
    scms: HashMap<String, Arc<dyn Scm>>,
    notifiers: HashMap<String, Arc<dyn Notifier>>,
    terminals: HashMap<String, Arc<dyn Terminal>>,
}

macro_rules! plugin_accessors {
    ($($field:ident, $trait:ident, $entity:literal, $register:ident, $get:ident);+ $(;)?) => {
        $(
            pub fn $register(&mut self, plugin: Arc<dyn $trait>) -> Result<(), EnnioError> {
                let name = plugin.name().to_owned();
                if self.$field.contains_key(&name) {
                    return Err(EnnioError::AlreadyExists {
                        entity: $entity.to_owned(),
                        id: name,
                    });
                }
                self.$field.insert(name, plugin);
                Ok(())
            }

            pub fn $get(&self, name: &str) -> Result<&Arc<dyn $trait>, EnnioError> {
                self.$field.get(name).ok_or_else(|| EnnioError::NotFound {
                    entity: $entity.to_owned(),
                    id: name.to_owned(),
                })
            }
        )+
    };
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
            agents: HashMap::new(),
            workspaces: HashMap::new(),
            trackers: HashMap::new(),
            scms: HashMap::new(),
            notifiers: HashMap::new(),
            terminals: HashMap::new(),
        }
    }

    plugin_accessors! {
        runtimes, Runtime, "runtime", register_runtime, get_runtime;
        agents, Agent, "agent", register_agent, get_agent;
        workspaces, Workspace, "workspace", register_workspace, get_workspace;
        trackers, Tracker, "tracker", register_tracker, get_tracker;
        scms, Scm, "scm", register_scm, get_scm;
        notifiers, Notifier, "notifier", register_notifier, get_notifier;
        terminals, Terminal, "terminal", register_terminal, get_terminal;
    }

    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        fn manifests_for<V>(
            map: &HashMap<String, V>,
            slot: PluginSlot,
        ) -> impl Iterator<Item = PluginManifest> + '_ {
            map.keys().map(move |name| PluginManifest {
                name: name.to_owned(),
                slot,
                version: String::new(),
                description: String::new(),
            })
        }

        manifests_for(&self.runtimes, PluginSlot::Runtime)
            .chain(manifests_for(&self.agents, PluginSlot::Agent))
            .chain(manifests_for(&self.workspaces, PluginSlot::Workspace))
            .chain(manifests_for(&self.trackers, PluginSlot::Tracker))
            .chain(manifests_for(&self.scms, PluginSlot::Scm))
            .chain(manifests_for(&self.notifiers, PluginSlot::Notifier))
            .chain(manifests_for(&self.terminals, PluginSlot::Terminal))
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::time::Duration;

    use ennio_core::agent::{AgentLaunchConfig, AgentSessionInfo, WorkspaceHooksConfig};
    use ennio_core::config::ProjectConfig;
    use ennio_core::runtime::{AttachInfo, RuntimeCreateConfig, RuntimeHandle, RuntimeMetrics};
    use ennio_core::session::{ActivityDetection, ActivityState, Session};
    use ennio_core::workspace::{WorkspaceCreateConfig, WorkspaceInfo};

    use super::*;

    struct StubRuntime;

    #[async_trait::async_trait]
    impl Runtime for StubRuntime {
        fn name(&self) -> &str {
            "stub-runtime"
        }
        async fn create(&self, _config: &RuntimeCreateConfig) -> Result<RuntimeHandle, EnnioError> {
            Err(EnnioError::Internal {
                message: "stub".to_owned(),
            })
        }
        async fn destroy(&self, _handle: &RuntimeHandle) -> Result<(), EnnioError> {
            Ok(())
        }
        async fn send_message(
            &self,
            _handle: &RuntimeHandle,
            _message: &str,
        ) -> Result<(), EnnioError> {
            Ok(())
        }
        async fn get_output(
            &self,
            _handle: &RuntimeHandle,
            _lines: u32,
        ) -> Result<String, EnnioError> {
            Ok(String::new())
        }
        async fn is_alive(&self, _handle: &RuntimeHandle) -> Result<bool, EnnioError> {
            Ok(true)
        }
        async fn get_metrics(&self, _handle: &RuntimeHandle) -> Result<RuntimeMetrics, EnnioError> {
            Ok(RuntimeMetrics {
                uptime: Duration::from_secs(0),
                cpu_percent: None,
                memory_bytes: None,
            })
        }
        async fn get_attach_info(&self, _handle: &RuntimeHandle) -> Result<AttachInfo, EnnioError> {
            Ok(AttachInfo {
                command: String::new(),
                url: None,
                instructions: None,
            })
        }
    }

    struct StubAgent;

    #[async_trait::async_trait]
    impl Agent for StubAgent {
        fn name(&self) -> &str {
            "stub-agent"
        }
        fn process_name(&self) -> &str {
            "stub"
        }
        fn prompt_delivery(&self) -> ennio_core::agent::PromptDelivery {
            ennio_core::agent::PromptDelivery::Inline
        }
        fn get_launch_command(&self, _config: &AgentLaunchConfig<'_>) -> String {
            String::new()
        }
        fn get_environment(&self, _config: &AgentLaunchConfig<'_>) -> HashMap<String, String> {
            HashMap::new()
        }
        fn detect_activity(&self, _terminal_output: &str) -> ActivityState {
            ActivityState::Idle
        }
        async fn get_activity_state(
            &self,
            _session: &Session,
            _ready_threshold: Duration,
        ) -> Result<Option<ActivityDetection>, EnnioError> {
            Ok(None)
        }
        async fn is_process_running(&self, _handle: &RuntimeHandle) -> Result<bool, EnnioError> {
            Ok(false)
        }
        async fn get_session_info(
            &self,
            _session: &Session,
        ) -> Result<Option<AgentSessionInfo>, EnnioError> {
            Ok(None)
        }
        async fn get_restore_command(
            &self,
            _session: &Session,
            _project: &ProjectConfig,
        ) -> Result<Option<String>, EnnioError> {
            Ok(None)
        }
        async fn post_launch_setup(&self, _session: &Session) -> Result<(), EnnioError> {
            Ok(())
        }
        async fn setup_workspace_hooks(
            &self,
            _workspace_path: &Path,
            _config: &WorkspaceHooksConfig<'_>,
        ) -> Result<(), EnnioError> {
            Ok(())
        }
    }

    struct StubWorkspace;

    #[async_trait::async_trait]
    impl Workspace for StubWorkspace {
        fn name(&self) -> &str {
            "stub-workspace"
        }
        async fn create(
            &self,
            _config: &WorkspaceCreateConfig<'_>,
        ) -> Result<std::path::PathBuf, EnnioError> {
            Ok(std::path::PathBuf::from("/tmp/stub"))
        }
        async fn destroy(&self, _path: &Path) -> Result<(), EnnioError> {
            Ok(())
        }
        async fn list(
            &self,
            _project_id: &ennio_core::id::ProjectId,
        ) -> Result<Vec<WorkspaceInfo>, EnnioError> {
            Ok(vec![])
        }
        async fn post_create(
            &self,
            _path: &Path,
            _config: &WorkspaceCreateConfig<'_>,
        ) -> Result<(), EnnioError> {
            Ok(())
        }
        async fn exists(&self, _path: &Path) -> Result<bool, EnnioError> {
            Ok(false)
        }
        async fn restore(
            &self,
            _path: &Path,
            _config: &WorkspaceCreateConfig<'_>,
        ) -> Result<(), EnnioError> {
            Ok(())
        }
    }

    #[test]
    fn register_and_get_runtime() {
        let mut registry = PluginRegistry::new();
        let runtime = Arc::new(StubRuntime) as Arc<dyn Runtime>;
        registry.register_runtime(runtime).unwrap();

        let fetched = registry.get_runtime("stub-runtime").unwrap();
        assert_eq!(fetched.name(), "stub-runtime");
    }

    #[test]
    fn duplicate_runtime_rejected() {
        let mut registry = PluginRegistry::new();
        let rt1 = Arc::new(StubRuntime) as Arc<dyn Runtime>;
        let rt2 = Arc::new(StubRuntime) as Arc<dyn Runtime>;
        registry.register_runtime(rt1).unwrap();
        assert!(registry.register_runtime(rt2).is_err());
    }

    #[test]
    fn get_missing_runtime_returns_not_found() {
        let registry = PluginRegistry::new();
        assert!(registry.get_runtime("nonexistent").is_err());
    }

    #[test]
    fn register_and_get_agent() {
        let mut registry = PluginRegistry::new();
        let agent = Arc::new(StubAgent) as Arc<dyn Agent>;
        registry.register_agent(agent).unwrap();

        let fetched = registry.get_agent("stub-agent").unwrap();
        assert_eq!(fetched.name(), "stub-agent");
    }

    #[test]
    fn register_and_get_workspace() {
        let mut registry = PluginRegistry::new();
        let ws = Arc::new(StubWorkspace) as Arc<dyn Workspace>;
        registry.register_workspace(ws).unwrap();

        let fetched = registry.get_workspace("stub-workspace").unwrap();
        assert_eq!(fetched.name(), "stub-workspace");
    }

    #[test]
    fn list_plugins_includes_all_registered() {
        let mut registry = PluginRegistry::new();
        registry
            .register_runtime(Arc::new(StubRuntime) as Arc<dyn Runtime>)
            .unwrap();
        registry
            .register_agent(Arc::new(StubAgent) as Arc<dyn Agent>)
            .unwrap();
        registry
            .register_workspace(Arc::new(StubWorkspace) as Arc<dyn Workspace>)
            .unwrap();

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 3);

        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"stub-runtime"));
        assert!(names.contains(&"stub-agent"));
        assert!(names.contains(&"stub-workspace"));
    }
}

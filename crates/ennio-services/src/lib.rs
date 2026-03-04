pub mod config_loader;
pub mod event_bus;
mod events;
pub mod lifecycle_manager;
pub mod registry;
pub mod session_manager;

pub use config_loader::{apply_project_defaults, find_config_file, load_config, validate_config};
pub use event_bus::EventBus;
pub use lifecycle_manager::DefaultLifecycleManager;
pub use registry::{PluginRegistry, register_default_plugins};
pub use session_manager::DefaultSessionManager;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PluginSlot {
    Runtime,
    Agent,
    Workspace,
    Tracker,
    Scm,
    Notifier,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub slot: PluginSlot,
    pub version: String,
    pub description: String,
}

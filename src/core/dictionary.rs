// src/core/dictionary.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CommandTemplate {
    pub name: String,
    #[serde(default)] 
    pub priority: String,
    #[serde(default)] 
    pub has_level: bool,
    #[serde(default)] 
    pub level: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PluginDictionary {
    #[serde(default)]
    pub status_codes: Vec<String>,
    #[serde(default)]
    pub command_templates: HashMap<String, CommandTemplate>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DirectiveDictionary {
    #[serde(default)]
    pub plugin_dictionaries: HashMap<String, PluginDictionary>,
}
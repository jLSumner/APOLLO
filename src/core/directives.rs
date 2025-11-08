// src/core/directives.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

///represents a single "if-this-then-that" rule.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Rule {
    pub if_status_is: String,
    pub then_command_target: String,
    pub then_command_json: String,
}

///Contains the directives specific to a single Entity.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EntityDirectives {
    #[serde(default)]
    pub directives: Vec<Rule>,
}

///Contains directives for a Subsection and any child Entities.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SubsectionDirectives {
    #[serde(default)]
    pub directives: Vec<Rule>,
    #[serde(default)]
    pub entities: HashMap<String, EntityDirectives>,
}

///Contains directives for a Plugin and any child Subsections.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PluginDirectives {
    #[serde(default)]
    pub directives: Vec<Rule>,
    #[serde(default)]
    pub subsections: HashMap<String, SubsectionDirectives>,
}

///the root of our directives.json file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DirectiveConfig {
    #[serde(default)]
    pub plugins: HashMap<String, PluginDirectives>,
}
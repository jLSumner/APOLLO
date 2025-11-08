// src/state/config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

///he final node in the hierarchy, representing a single connecting client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entity {
    pub auth_key: String,
}

///A collection of entities, representing a logical grouping within a plugin.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Subsection {
    pub auth_key: String,
    #[serde(default)]
    pub entities: HashMap<String, Entity>,
}

///The top-level category for a group of subsections and entities.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Plugin {
    pub auth_key: String,
    #[serde(default)]
    pub subsections: HashMap<String, Subsection>,
}

impl Plugin {
    pub fn new(auth_key: String) -> Self {
        Self {
            auth_key,
            subsections: HashMap::new(),
        }
    }
}

impl Subsection {
    pub fn new(auth_key: String) -> Self {
        Self {
            auth_key,
            entities: HashMap::new(),
        }
    }
}

/// The root of the configuration file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub plugins: HashMap<String, Plugin>,
}
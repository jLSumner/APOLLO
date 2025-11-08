// src/core/engine.rs

use super::directives::{DirectiveConfig, Rule};
use std::io;
use log;

#[derive(Debug)]
pub struct DirectiveEngine {
    config: DirectiveConfig,
}

impl DirectiveEngine {
    pub fn new(config: DirectiveConfig) -> Self {
        Self { config }
    }

    pub fn get_config(&self) -> &DirectiveConfig {
        &self.config
    }

    pub async fn add_and_save_rule(&mut self, target_id: &str, rule: Rule) -> io::Result<()> {
        let id_parts: Vec<&str> = target_id.split('_').collect();
        
        let plugin_id = *id_parts.get(0).unwrap_or(&"");
        let subsection_id = id_parts.get(1);
        let entity_id = id_parts.get(2);

        let plugin = self.config.plugins.entry(plugin_id.to_string()).or_default();

        if let (Some(sub_id), Some(ent_id)) = (subsection_id, entity_id) {
            plugin.subsections.entry(sub_id.to_string()).or_default()
                .entities.entry(ent_id.to_string()).or_default()
                .directives.push(rule);
        } else if let Some(sub_id) = subsection_id {
            plugin.subsections.entry(sub_id.to_string()).or_default()
                .directives.push(rule);
        } else if !plugin_id.is_empty() {
            plugin.directives.push(rule);
        }
        
        self.save_to_disk().await
    }
    
    pub async fn remove_and_save_rule(&mut self, target_id: &str, rule_to_remove: &Rule) -> io::Result<()> {
        let id_parts: Vec<&str> = target_id.split('_').collect();
        
        let plugin_id = id_parts.get(0).cloned().unwrap_or("");
        let subsection_id = id_parts.get(1).cloned();
        let entity_id = id_parts.get(2).cloned();

        let mut found_and_removed = false;
        if let Some(plugin) = self.config.plugins.get_mut(plugin_id) {
            match (subsection_id, entity_id) {
                (Some(sub_id), Some(ent_id)) => {
                    // Entity Level.
                    if let Some(sub) = plugin.subsections.get_mut(sub_id) {
                        if let Some(ent) = sub.entities.get_mut(ent_id) {
                            let initial_len = ent.directives.len();
                            ent.directives.retain(|r| r != rule_to_remove);
                            if ent.directives.len() < initial_len { found_and_removed = true; }
                        }
                    }
                },
                (Some(sub_id), None) => {
                    //Subsection Level.
                    if let Some(sub) = plugin.subsections.get_mut(sub_id) {
                        let initial_len = sub.directives.len();
                        sub.directives.retain(|r| r != rule_to_remove);
                        if sub.directives.len() < initial_len { found_and_removed = true; }
                    }
                },
                (None, None) => {
                    //plugin Level.
                    let initial_len = plugin.directives.len();
                    plugin.directives.retain(|r| r != rule_to_remove);
                    if plugin.directives.len() < initial_len { found_and_removed = true; }
                },
                _ => {} 
            }
        }
        
        if found_and_removed {
            log::info!("[CORE] Removing directive for target '{}'", target_id);
            self.save_to_disk().await
        } else {
            log::warn!("[CORE] Could not find directive to remove for target '{}'", target_id);
            Ok(())
        }
    }

    async fn save_to_disk(&self) -> io::Result<()> {
        let json_string = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::write("directives.json", json_string).await?;
        log::info!("[CORE] Successfully saved updated directives to directives.json");
        Ok(())
    }
    
    pub fn process_report(&self, entity_id_str: &str, status: &str) -> Option<(String, String)> {
        let id_parts: Vec<&str> = entity_id_str.split('_').collect();
        if id_parts.len() != 3 {
            return None;
        }
        let (plugin_id, subsection_id, entity_id) = (id_parts[0], id_parts[1], id_parts[2]);

        if let Some(rule) = self.config.plugins.get(plugin_id)
            .and_then(|p| p.subsections.get(subsection_id))
            .and_then(|s| s.entities.get(entity_id))
            .and_then(|e| find_rule_in_vec(&e.directives, status))
        {
            return Some((rule.then_command_target.clone(), rule.then_command_json.clone()));
        }

        if let Some(rule) = self.config.plugins.get(plugin_id)
            .and_then(|p| p.subsections.get(subsection_id))
            .and_then(|s| find_rule_in_vec(&s.directives, status))
        {
            return Some((rule.then_command_target.clone(), rule.then_command_json.clone()));
        }

        if let Some(rule) = self.config.plugins.get(plugin_id)
            .and_then(|p| find_rule_in_vec(&p.directives, status))
        {
            return Some((rule.then_command_target.clone(), rule.then_command_json.clone()));
        }

        None
    }
}

fn find_rule_in_vec<'a>(rules: &'a [Rule], status: &str) -> Option<&'a Rule> {
    rules.iter().find(|r| r.if_status_is == status)
}
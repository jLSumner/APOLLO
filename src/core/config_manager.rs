// src/core/config_manager.rs

use crate::state::config::{Config, Entity, Plugin, Subsection};
use std::io;
use log;

#[derive(Debug, Clone)]
pub struct ConfigManager {
    pub config: Config,
}

impl ConfigManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn add_plugin(&mut self, plugin_id: String, auth_key: String) -> io::Result<()> {
        if self.config.plugins.contains_key(&plugin_id) {
            log::warn!("[CONFIG] Plugin '{}' already exists. No changes made.", plugin_id);
            return Ok(());
        }

        let new_plugin = Plugin::new(auth_key);
        self.config.plugins.insert(plugin_id.clone(), new_plugin);
        log::info!("[CONFIG] Added new plugin '{}'.", plugin_id);
        self.save_to_disk().await
    }

    pub async fn add_subsection(
        &mut self,
        parent_plugin_id: &str,
        subsection_id: String,
        auth_key: String,
    ) -> io::Result<()> {
        if let Some(plugin) = self.config.plugins.get_mut(parent_plugin_id) {
            if plugin.subsections.contains_key(&subsection_id) {
                log::warn!("[CONFIG] Subsection '{}' already exists in plugin '{}'. No changes made.", subsection_id, parent_plugin_id);
                return Ok(());
            }

            let new_subsection = Subsection::new(auth_key);
            plugin.subsections.insert(subsection_id.clone(), new_subsection);
            log::info!("[CONFIG] Added new subsection '{}' to plugin '{}'.", subsection_id, parent_plugin_id);
            self.save_to_disk().await
        } else {
            log::error!("[CONFIG] Could not find parent plugin '{}' to add subsection to.", parent_plugin_id);
            Ok(())
        }
    }

    pub async fn add_entity(
        &mut self,
        parent_id: &str,
        entity_id: String,
        auth_key: String,
    ) -> io::Result<()> {
        let id_parts: Vec<&str> = parent_id.split('_').collect();
        if id_parts.len() != 2 {
            log::error!("[CONFIG] Invalid parent ID format '{}' for new entity.", parent_id);
            return Ok(());
        }
        let (plugin_id, subsection_id) = (id_parts[0], id_parts[1]);

        if let Some(plugin) = self.config.plugins.get_mut(plugin_id) {
            if let Some(subsection) = plugin.subsections.get_mut(subsection_id) {
                if subsection.entities.contains_key(&entity_id) {
                    log::warn!("[CONFIG] Entity '{}' already exists in '{}'. No changes made.", entity_id, parent_id);
                    return Ok(());
                }

                let new_entity = Entity { auth_key };
                subsection.entities.insert(entity_id.clone(), new_entity);
                log::info!("[CONFIG] Added new entity '{}' to '{}'.", entity_id, parent_id);
                self.save_to_disk().await
            } else {
                 log::error!("[CONFIG] Could not find parent subsection '{}' to add entity to.", parent_id);
                 Ok(())
            }
        } else {
            log::error!("[CONFIG] Could not find parent plugin '{}' to add entity to.", plugin_id);
            Ok(())
        }
    }

    // Function to remove a top-level plugin,
    pub async fn remove_plugin(&mut self, plugin_id: &str) -> io::Result<()> {
        if self.config.plugins.remove(plugin_id).is_some() {
            log::info!("[CONFIG] Removed plugin '{}'.", plugin_id);
            self.save_to_disk().await
        } else {
            log::warn!("[CONFIG] Could not find plugin '{}' to remove.", plugin_id);
            Ok(())
        }
    }

    // function to remove a subsection.
    pub async fn remove_subsection(&mut self, plugin_id: &str, subsection_id: &str) -> io::Result<()> {
        if let Some(plugin) = self.config.plugins.get_mut(plugin_id) {
            if plugin.subsections.remove(subsection_id).is_some() {
                log::info!("[CONFIG] Removed subsection '{}' from plugin '{}'.", subsection_id, plugin_id);
                self.save_to_disk().await
            } else {
                log::warn!("[CONFIG] Could not find subsection '{}' to remove.", subsection_id);
                Ok(())
            }
        } else {
            log::warn!("[CONFIG] Could not find parent plugin '{}' for removal.", plugin_id);
            Ok(())
        }
    }

    // Function to remove an entity.
    pub async fn remove_entity(&mut self, parent_id: &str, entity_id: &str) -> io::Result<()> {
        let id_parts: Vec<&str> = parent_id.split('_').collect();
        if id_parts.len() != 2 { return Ok(()); }
        let (plugin_id, subsection_id) = (id_parts[0], id_parts[1]);
        
        if let Some(plugin) = self.config.plugins.get_mut(plugin_id) {
            if let Some(subsection) = plugin.subsections.get_mut(subsection_id) {
                if subsection.entities.remove(entity_id).is_some() {
                    log::info!("[CONFIG] Removed entity '{}' from '{}'.", entity_id, parent_id);
                    self.save_to_disk().await
                } else {
                    log::warn!("[CONFIG] Could not find entity '{}' to remove.", entity_id);
                    Ok(())
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    async fn save_to_disk(&self) -> io::Result<()> {
        let json_string = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::write("config.json", json_string).await?;
        log::info!("[CORE] Successfully saved updated plugin config to config.json");
        Ok(())
    }
}
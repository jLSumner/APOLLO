// src/core/dictionary_manager.rs

use crate::core::dictionary::{CommandTemplate, DirectiveDictionary};
use std::io;
use log;

#[derive(Debug, Clone)]
pub struct DictionaryManager {
    pub dictionary: DirectiveDictionary,
}

impl DictionaryManager {
    pub fn new(dictionary: DirectiveDictionary) -> Self {
        Self { dictionary }
    }

    pub async fn add_status_code(&mut self, plugin_id: &str, new_code: String) -> io::Result<()> {
        let plugin_dict = self.dictionary.plugin_dictionaries.entry(plugin_id.to_string()).or_default();
        if !plugin_dict.status_codes.contains(&new_code) {
            plugin_dict.status_codes.push(new_code);
            log::info!("[DICT] Added status code to '{}'", plugin_id);
            self.save_to_disk().await?;
        } else {
            log::warn!("[DICT] Status code already exists for '{}'. No changes made.", plugin_id);
        }
        Ok(())
    }

    pub async fn add_command_template(
        &mut self,
        plugin_id: &str,
        template_key: String,
        template: CommandTemplate,
    ) -> io::Result<()> {
        let plugin_dict = self.dictionary.plugin_dictionaries.entry(plugin_id.to_string()).or_default();
        if !plugin_dict.command_templates.contains_key(&template_key) {
            plugin_dict.command_templates.insert(template_key.clone(), template);
            log::info!("[DICT] Added command template '{}' to '{}'", template_key, plugin_id);
            self.save_to_disk().await?;
        } else {
            log::warn!("[DICT] Command template key '{}' already exists for '{}'. No changes made.", template_key, plugin_id);
        }
        Ok(())
    }

    pub async fn remove_status_code(&mut self, plugin_id: &str, code_to_remove: &str) -> io::Result<()> {
        if let Some(plugin_dict) = self.dictionary.plugin_dictionaries.get_mut(plugin_id) {
            let initial_len = plugin_dict.status_codes.len();
            plugin_dict.status_codes.retain(|code| code != code_to_remove);
            
            if plugin_dict.status_codes.len() < initial_len {
                log::info!("[DICT] Removed status code '{}' from '{}'", code_to_remove, plugin_id);
                self.save_to_disk().await?;
            } else {
                log::warn!("[DICT] Status code '{}' not found for '{}'. No changes made.", code_to_remove, plugin_id);
            }
        } else {
            log::warn!("[DICT] Plugin group '{}' not found for status code removal.", plugin_id);
        }
        Ok(())
    }
	
	pub async fn remove_command_template(&mut self, plugin_id: &str, key_to_remove: &str) -> io::Result<()> {
    if let Some(plugin_dict) = self.dictionary.plugin_dictionaries.get_mut(plugin_id) {
        if plugin_dict.command_templates.remove(key_to_remove).is_some() {
            log::info!("[DICT] Removed command template '{}' from '{}'", key_to_remove, plugin_id);
            self.save_to_disk().await?;
        } else {
            log::warn!("[DICT] Command template key '{}' not found for '{}'. No changes made.", key_to_remove, plugin_id);
        }
    } else {
        log::warn!("[DICT] Plugin group '{}' not found for command removal.", plugin_id);
    }
    Ok(())
}

    async fn save_to_disk(&self) -> io::Result<()> {
        let json_string = serde_json::to_string_pretty(&self.dictionary)?;
        tokio::fs::write("directive_dictionary.json", json_string).await?;
        log::info!("[CORE] Successfully saved updated directive dictionary to disk.");
        Ok(())
    }
}
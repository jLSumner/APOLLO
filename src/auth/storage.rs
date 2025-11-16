// src/auth/storage.rs

use super::types::AuthConfig;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};
use std::path::Path;

const AUTH_CONFIG_FILE: &str = "apollo_auth.enc";

// Derive encryption key from system information
fn get_encryption_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Use hostname as part of key (different on each system)
    if let Ok(hostname) = hostname::get() {
        hasher.update(hostname.to_string_lossy().as_bytes());
    }
    
    // Add a constant salt
    hasher.update(b"APOLLO-AUTH-CONFIG-SALT-V1-2025");
    
    let result = hasher.finalize();
    result.into()
}

pub fn save_auth_config(config: &AuthConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize to JSON
    let json = serde_json::to_string_pretty(config)?;
    
    // Encrypt
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new(&key.into());
    let nonce = Nonce::from_slice(b"unique nonce"); // In production, use random nonce
    
    let ciphertext = cipher.encrypt(nonce, json.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Save to file
    std::fs::write(AUTH_CONFIG_FILE, ciphertext)?;
    
    log::info!("[AUTH] Configuration saved successfully");
    Ok(())
}

pub fn load_auth_config() -> Result<AuthConfig, Box<dyn std::error::Error>> {
    if !Path::new(AUTH_CONFIG_FILE).exists() {
        return Err("Auth config file not found - run initial setup".into());
    }
    
    // Read encrypted file
    let ciphertext = std::fs::read(AUTH_CONFIG_FILE)?;
    
    // Decrypt
    let key = get_encryption_key();
    let cipher = Aes256Gcm::new(&key.into());
    let nonce = Nonce::from_slice(b"unique nonce");
    
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;
    
    // Deserialize
    let config: AuthConfig = serde_json::from_slice(&plaintext)?;
    
    Ok(config)
}

pub fn config_exists() -> bool {
    Path::new(AUTH_CONFIG_FILE).exists()
}
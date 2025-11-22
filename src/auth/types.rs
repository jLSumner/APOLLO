// src/auth/types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Administrator {
    pub username: String,
    pub password_hash: String,  // ← No attribute here
    pub full_name: String,
    pub created_at: String,
    pub last_login: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthConfig {
    pub administrators: HashMap<String, Administrator>,
    pub session_timeout_minutes: u64,
    pub max_failed_attempts: u32,
}

impl AuthConfig {
    pub fn new() -> Self {
        Self {
            administrators: HashMap::new(),
            session_timeout_minutes: 60,
            max_failed_attempts: 5,
        }
    }
    
    pub fn add_administrator(&mut self, admin: Administrator) {
        self.administrators.insert(admin.username.clone(), admin);
    }
    
    pub fn verify_credentials(&self, username: &str, password: &str) -> bool {
        if let Some(admin) = self.administrators.get(username) {
            bcrypt::verify(password, &admin.password_hash).unwrap_or(false)
        } else {
            // Still run bcrypt to prevent timing attacks
            let _ = bcrypt::verify(password, "$2b$12$dummy.hash.to.prevent.timing.attacks.here");
            false
        }
    }
    
    pub fn update_last_login(&mut self, username: &str) {
        if let Some(admin) = self.administrators.get_mut(username) {
            admin.last_login = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}
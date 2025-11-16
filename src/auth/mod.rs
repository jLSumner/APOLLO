// src/auth/mod.rs

pub mod types;
pub mod storage;
pub mod setup;

pub use types::{Administrator, AuthConfig};
pub use storage::{save_auth_config, load_auth_config, config_exists};
pub use setup::run_initial_setup;
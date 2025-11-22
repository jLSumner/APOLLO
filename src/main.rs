// src/main.rs

mod state;
mod ui;
mod logging;
mod core;
mod grpc;
mod auth;  // ← ADDED

use state::app_state::ApolloState;
use state::config::Config;
use core::config_manager::ConfigManager;
use core::directives::DirectiveConfig;
use core::engine::DirectiveEngine;
use core::dictionary::DirectiveDictionary;
use core::dictionary_manager::DictionaryManager;
use core::security::SecurityCodes;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;
use grpc::ccp::core_connector_server::CoreConnectorServer;
use grpc::ApolloGrpcService;
use log;
use chrono::{Utc, NaiveDate};

// A dedicated function for the cleanup task...
fn run_log_cleanup_task() {
    log::info!("[CORE] Running daily log cleanup task...");
    if fs::create_dir_all("logs").is_err() {
        log::error!("[CORE] Could not create logs directory for cleanup.");
        return;
    }

    let Ok(paths) = fs::read_dir("./logs") else {
        log::warn!("[CORE] Could not read logs directory for cleanup.");
        return;
    };

    let today = Utc::now().date_naive();
    let four_weeks_ago = today - chrono::Duration::weeks(4);

    for path in paths.flatten() {
        let file_name = path.file_name().into_string().unwrap_or_default();
        if file_name.starts_with("APOLLO-") && file_name.ends_with(".log") {
            if let Some(date_str) = file_name.strip_prefix("APOLLO-").and_then(|s| s.strip_suffix(".log")) {
                if let Ok(file_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    if file_date < four_weeks_ago {
                        if fs::remove_file(path.path()).is_ok() {
                            log::info!("[CORE] Deleted old log file: {}", file_name);
                        } else {
                            log::error!("[CORE] Failed to delete old log file: {}", file_name);
                        }
                    }
                }
            }
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_buffer = Arc::new(Mutex::new(Vec::new()));
    logging::initialize_logging(log_buffer.clone())?;
    
    run_log_cleanup_task();
    
    log::info!("[CORE] Loading APOLLO plugin configuration...");
    let config_str = fs::read_to_string("config.json")?;
    let config: Config = serde_json::from_str(&config_str)?;
    log::info!("[CORE] Plugin configuration loaded successfully.");
    
    log::info!("[CORE] Loading APOLLO directive configuration...");
    let directives_str = fs::read_to_string("directives.json")?;
    let directive_config: DirectiveConfig = serde_json::from_str(&directives_str)?;
    log::info!("[CORE] Directive configuration loaded successfully.");

    log::info!("[CORE] Loading APOLLO directive dictionary...");
    let dictionary_str = fs::read_to_string("directive_dictionary.json")?;
    let dictionary_data: DirectiveDictionary = serde_json::from_str(&dictionary_str)?;
    log::info!("[CORE] Directive dictionary loaded successfully.");

    log::info!("[CORE] Loading APOLLO security codes...");
    let security_codes = Arc::new(SecurityCodes::load_from_file("security_codes.txt")?);
    log::info!("[CORE] Security codes loaded successfully.");

    // ========== AUTHENTICATION SETUP (NEW) ==========
    log::info!("[CORE] Checking authentication configuration...");
    let auth_config = if auth::config_exists() {
        log::info!("[CORE] Loading existing authentication configuration...");
        auth::load_auth_config()?
    } else {
        log::warn!("[CORE] No authentication configuration found.");
        log::warn!("[CORE] Starting initial administrator setup...");
        println!("\n"); // Add some space before the setup wizard
        let config = auth::run_initial_setup()?;
        auth::save_auth_config(&config)?;
        log::info!("[CORE] Authentication configuration saved.");
        config
    };
    let auth_config = Arc::new(TokioMutex::new(auth_config));
    // ================================================

    let config_manager = Arc::new(TokioMutex::new(ConfigManager::new(config)));
    let dictionary_manager = Arc::new(TokioMutex::new(DictionaryManager::new(dictionary_data)));
    
    let app_state = Arc::new(TokioMutex::new(ApolloState::default()));
    let engine = Arc::new(TokioMutex::new(DirectiveEngine::new(directive_config)));
    
    let grpc_config_manager = config_manager.clone();
    let grpc_app_state = app_state.clone();
    let grpc_engine = engine.clone();
    tokio::spawn(async move {
        let addr = "[::1]:50051".parse().unwrap();
        log::info!("[gRPC] Server listening on {}", addr);
        let apollo_service = ApolloGrpcService::new(grpc_config_manager, grpc_app_state, grpc_engine);
        tonic::transport::Server::builder()
            .add_service(CoreConnectorServer::new(apollo_service))
            .serve(addr)
            .await
            .unwrap();
    });

    let monitor_app_state = app_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let timeout_duration = Duration::from_secs(15);
            let mut state = monitor_app_state.lock().await;
            state.active_plugins.retain(|_token, plugin| {
                if plugin.state.last_seen.elapsed() > timeout_duration {
                    log::warn!("[MONITOR] Plugin '{}' timed out. Removing from active list.", plugin.state.entity_id);
                    false
                } else {
                    true
                }
            });
        }
    });

    // UPDATED: Pass auth_config to UI
    ui::run(log_buffer, app_state, engine, dictionary_manager, config_manager, security_codes, auth_config).await?;

    Ok(())
}
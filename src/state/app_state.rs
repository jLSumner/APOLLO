// src/state/app_state.rs

use crate::grpc::ccp::HeartbeatResponse;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::mpsc;
use tonic::Status;

///holds the live state of a single connected plugin.
#[derive(Debug, Clone)]
pub struct PluginState {
    pub entity_id: String,
    pub session_token: String,
    pub status: String,
    pub last_seen: Instant,
}

//struct to hold all information about an active plugin session.
#[derive(Debug)]
pub struct ActivePlugin {
    pub state: PluginState,
    pub command_sender: mpsc::Sender<Result<HeartbeatResponse, Status>>,
}

///The central, shared state of the entire APOLLO application.
#[derive(Debug, Default)]
pub struct ApolloState {
    /// A map of session_token -> ActivePlugin for all active plugins.
    pub active_plugins: HashMap<String, ActivePlugin>,
}
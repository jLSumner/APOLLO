//src/grpc/mod.rs

use crate::core::config_manager::ConfigManager;
use crate::core::engine::DirectiveEngine;
use crate::state::app_state::{ActivePlugin, ApolloState, PluginState};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use log;

pub mod ccp {
    tonic::include_proto!("ccp");
}

use ccp::{
    core_connector_server::CoreConnector, HandshakeRequest, HandshakeResponse, HeartbeatResponse,
    StatusReport,
};

#[derive(Debug)]
pub struct ApolloGrpcService {
    config_manager: Arc<Mutex<ConfigManager>>,
    app_state: Arc<Mutex<ApolloState>>,
    engine: Arc<Mutex<DirectiveEngine>>,
}

impl ApolloGrpcService {
    pub fn new(
        config_manager: Arc<Mutex<ConfigManager>>,
        app_state: Arc<Mutex<ApolloState>>,
        engine: Arc<Mutex<DirectiveEngine>>,
    ) -> Self {
        Self {
            config_manager,
            app_state,
            engine,
        }
    }
}

#[tonic::async_trait]
impl CoreConnector for ApolloGrpcService {
    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        let req = request.into_inner();
        log::info!("[gRPC] Received handshake from entity: {}", req.entity_id);

        let id_parts: Vec<&str> = req.entity_id.split('_').collect();
        let manager = self.config_manager.lock().await;
        let config = &manager.config;
        
        let auth_ok = match id_parts.len() {
            // case 1: Full Entity ID (e.g., ARC_MED_DOOR001).
            3 => {
                let (plugin_id, subsection_id, entity_id) = (id_parts[0], id_parts[1], id_parts[2]);
                config
                    .plugins
                    .get(plugin_id)
                    .and_then(|p| p.subsections.get(subsection_id))
                    .and_then(|s| s.entities.get(entity_id))
                    .map_or(false, |e| e.auth_key == req.auth_key)
            }
            //Case 2: Plugin-level ID (e.g., ARC).
            1 => {
                let plugin_id = id_parts[0];
                config
                    .plugins
                    .get(plugin_id)
                    .map_or(false, |p| p.auth_key == req.auth_key)
            }
            _ => {
                log::warn!("[gRPC] Handshake failed: Invalid entity ID format '{}'", req.entity_id);
                return Err(Status::invalid_argument("Entity ID format is invalid."));
            }
        };

        if auth_ok {
            log::info!("[gRPC] Authentication successful for {}", req.entity_id);
            let session_token = uuid::Uuid::new_v4().to_string();
            let (tx, _rx) = mpsc::channel(4);
            
            let mut app_state = self.app_state.lock().await;

            let new_plugin = ActivePlugin {
                state: PluginState {
                    entity_id: req.entity_id.clone(),
                    session_token: session_token.clone(),
                    status: "Authenticated".to_string(),
                    last_seen: Instant::now(),
                },
                command_sender: tx,
            };

            app_state.active_plugins.insert(session_token.clone(), new_plugin);
            log::info!("[STATE] Added '{}' to active plugins. Total: {}", req.entity_id, app_state.active_plugins.len());

            let response = HandshakeResponse {
                session_token,
                message: "Authentication successful.".to_string(),
            };
            Ok(Response::new(response))
        } else {
            log::warn!("[gRPC] Authentication failed for {}: Invalid ID or key.", req.entity_id);
            Err(Status::unauthenticated("Invalid credentials provided."))
        }
    }

    type ReportStatusStream = ReceiverStream<Result<HeartbeatResponse, Status>>;

    async fn report_status(
        &self,
        request: Request<Streaming<StatusReport>>,
    ) -> Result<Response<Self::ReportStatusStream>, Status> {
        let mut inbound_stream = request.into_inner();
        let app_state = self.app_state.clone();
        let engine = self.engine.clone();
        let (tx, rx) = mpsc::channel(32);
        let session_token = if let Some(Ok(first_report)) = inbound_stream.next().await {
            let mut state = app_state.lock().await;
            if let Some(plugin) = state.active_plugins.get_mut(&first_report.session_token) {
                plugin.command_sender = tx.clone();
                log::info!("[gRPC] Command channel registered for '{}'", plugin.state.entity_id);
            }
            first_report.session_token
        } else {
            return Err(Status::invalid_argument("First status report is required."));
        };

        tokio::spawn(async move {
            let first_report = StatusReport { session_token: session_token.clone(), ..Default::default() };
            let mut full_stream = tokio_stream::once(Ok(first_report)).chain(inbound_stream);

            while let Some(result) = full_stream.next().await {
                if let Ok(report) = result {
                    let engine_guard = engine.lock().await;
                    let mut state_guard = app_state.lock().await;

                    if let Some(plugin) = state_guard.active_plugins.get_mut(&report.session_token) {
                        let entity_id = plugin.state.entity_id.clone();
                        
                        plugin.state.last_seen = Instant::now();

                        if !report.status.is_empty() {
                            log::info!("[gRPC] Status from '{}': {}", entity_id, report.status);
                            plugin.state.status = report.status.clone();

                            if let Some((target, command)) = engine_guard.process_report(&entity_id, &report.status) {
                                log::info!("[DIRECTIVE] Triggered for '{}' -> Command: {} -> Target: {}", entity_id, command, target);
                                
                                let mut target_sender: Option<mpsc::Sender<_>> = None;
                                for p in state_guard.active_plugins.values() {
                                    if p.state.entity_id == target {
                                        target_sender = Some(p.command_sender.clone());
                                        break;
                                    }
                                }
                                
                                if let Some(sender) = target_sender {
                                    let response = HeartbeatResponse { status: "CommandIssued".to_string(), command_json: command };
                                    if sender.try_send(Ok(response)).is_err() {
                                        log::warn!("[gRPC] Target command channel for '{}' was closed or full.", target);
                                    }
                                } else {
                                    log::warn!("[gRPC] Could not find active command channel for target '{}'.", target);
                                }
                            }
                        }
                    } else {
                        log::warn!("[gRPC] Received status from unknown session token. Terminating stream.");
                        break;
                    }
                } else {
                    log::error!("[gRPC] Plugin stream error.");
                    break;
                }
            }
            let mut state = app_state.lock().await;
            if let Some(plugin) = state.active_plugins.remove(&session_token) {
                log::info!("[gRPC] Plugin '{}' disconnected. Removing from active state.", plugin.state.entity_id);
            }
        });
        let outbound_stream = ReceiverStream::new(rx);
        Ok(Response::new(outbound_stream))
    }
}

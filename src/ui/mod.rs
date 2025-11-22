// src/ui/mod.rs

mod login;
mod splash;
mod views;

use crate::core::config_manager::ConfigManager;
use crate::core::dictionary::{CommandTemplate, DirectiveDictionary};
use crate::core::dictionary_manager::DictionaryManager;
use crate::core::directives::{DirectiveConfig, Rule};
use crate::core::engine::DirectiveEngine;
use crate::core::security::SecurityCodes;
use crate::logging::LogBuffer;
use crate::state::config::Config;
use crate::state::{
    app_state::{ApolloState, PluginState},
    App, AppMode, AuthenticatedMode, CommandFormFocus, DictionaryColumn, DictionaryEditorMode,
    DirectiveMode, DirectiveFormStep, LoginFocus, LogViewerMode, PluginMgmtFormStep,
    PluginMgmtMode, WipAction,
};
use crate::ui::views::directives::NodeType;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::{
    fs as StdFs,
    io::{self, BufRead, BufReader},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex as TokioMutex;
use tui_input::backend::crossterm::EventHandler;
use log;

pub async fn run(
    log_buffer: LogBuffer,
    app_state: Arc<TokioMutex<ApolloState>>,
    engine: Arc<TokioMutex<DirectiveEngine>>,
    dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
    config_manager: Arc<TokioMutex<ConfigManager>>,
    security_codes: Arc<SecurityCodes>,
	auth_config: Arc<TokioMutex<crate::auth::AuthConfig>>,
) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let splash_start_time = Instant::now();

    loop {
        if let Some((_, time_set)) = &app.status_message {
            if time_set.elapsed() > Duration::from_secs(4) {
                app.status_message = None;
            }
        }

        let log_items: Vec<ListItem> = {
            let log_guard = log_buffer.lock().unwrap();
            log_guard
                .iter()
                .map(|(_instant, log_str)| {
                    let mut spans = Vec::new();
                    let mut current_pos = 0;
                    let tags = [
                        "[CORE]",
                        "[gRPC]",
                        "[AUTH]",
                        "[STATE]",
                        "[DIRECTIVE]",
                        "[MONITOR]",
                        "[CONFIG]",
                    ];
                    let mut tag_found = false;

                    for tag in tags {
                        if let Some(tag_pos) = log_str.find(tag) {
                            let style = match tag {
                                "[DIRECTIVE]" => Style::default()
                                    .fg(Color::Red)
                                    .add_modifier(Modifier::BOLD),
                                "[STATE]" => Style::default().fg(Color::Magenta),
                                "[AUTH]" => Style::default().fg(Color::LightBlue),
                                "[gRPC]" => Style::default().fg(Color::Yellow),
                                "[CONFIG]" => Style::default().fg(Color::Blue),
                                "[CORE]" => Style::default().fg(Color::Green),
                                "[MONITOR]" => Style::default().fg(Color::Cyan),
                                _ => Style::default(),
                            };

                            spans.push(Span::raw(log_str[current_pos..tag_pos].to_string()));
                            spans.push(Span::styled(tag.to_string(), style));
                            current_pos = tag_pos + tag.len();
                            tag_found = true;
                            break;
                        }
                    }

                    spans.push(Span::raw(log_str[current_pos..].to_string()));

                    if tag_found {
                        ListItem::new(Line::from(spans))
                    } else {
                        ListItem::new(log_str.clone())
                    }
                })
                .collect()
        };

        let config_data_guard = config_manager.lock().await;
        let engine_guard = engine.lock().await;
        let dictionary_guard = dictionary_manager.lock().await;
        let app_state_guard = app_state.lock().await;

        let directives = engine_guard.get_config().clone();
        let plugins: Vec<PluginState> = app_state_guard
            .active_plugins
            .values()
            .map(|p| p.state.clone())
            .collect();

        if app.auth_mode == AuthenticatedMode::LogView
            && app.log_viewer_mode == LogViewerMode::Browser
            && app.log_files.is_empty()
        {
            app.log_files = get_log_files().unwrap_or_default();
        }

        terminal.draw(|f| {
            ui(
                f,
                &mut app,
                &log_items,
                &plugins,
                &directives,
                &config_data_guard.config,
                &dictionary_guard.dictionary,
            )
        })?;

        if app.mode == AppMode::SplashScreen && splash_start_time.elapsed() > Duration::from_secs(3)
        {
            app.mode = AppMode::Login;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_press(
                        &mut app,
                        key,
                        &directives,
                        &dictionary_guard.dictionary,
                        engine.clone(),
                        &config_data_guard.config,
                        config_manager.clone(),
                        &log_buffer,
                        dictionary_manager.clone(),
                        security_codes.clone(),
						auth_config.clone(), 
                    );
                    if key.code == KeyCode::Esc {
                        break;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn handle_key_press(
    app: &mut App,
    key: KeyEvent,
    directives: &DirectiveConfig,
    dictionary: &DirectiveDictionary,
    engine: Arc<TokioMutex<DirectiveEngine>>,
    config: &Config,
    config_manager: Arc<TokioMutex<ConfigManager>>,
    log_buffer: &LogBuffer,
    dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
    security_codes: Arc<SecurityCodes>,
	auth_config: Arc<TokioMutex<crate::auth::AuthConfig>>,
) {
    if app.wip_action.is_some() {
        handle_secure_confirmation(app, key, engine, config_manager, dictionary_manager, security_codes);
        return;
    }

    match app.mode {
        AppMode::Login => handle_login_keys(app, key, &auth_config),
        AppMode::Authenticated => handle_authenticated_keys(
            app,
            key,
            directives,
            dictionary,
            engine,
            config,
            config_manager,
            log_buffer,
            dictionary_manager,
        ),
        _ => {}
    }
}

fn handle_login_keys(
    app: &mut App,
    key: KeyEvent,
    auth_config: &Arc<TokioMutex<crate::auth::AuthConfig>>,
) {
    match key.code {
        KeyCode::Enter => {
            let username = app.username_input.value().to_string();
            let password = app.password_input.value().to_string();
            
            // We need to check auth but we're in a sync context
            // Temporarily use a simple approach: spawn task and check later
            // For now, use polling approach with try_lock
            
            if let Ok(mut config) = auth_config.try_lock() {
                let is_valid = config.verify_credentials(&username, &password);
                
                if is_valid {
                    log::info!("[AUTH] Successful login for user '{}'", username);
                    config.update_last_login(&username);
                    
                    // Save updated config (spawn task for async save)
                    let config_clone = config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = crate::auth::save_auth_config(&config_clone) {
                            log::error!("[AUTH] Failed to save login time: {}", e);
                        }
                    });
                    
                    app.mode = AppMode::Authenticated;
                    app.login_error = None;
                } else {
                    log::warn!("[AUTH] Failed login attempt for user '{}'", username);
                    app.login_error = Some("ACCESS DENIED".to_string());
                }
            } else {
                // Couldn't acquire lock - shouldn't happen but handle gracefully
                log::error!("[AUTH] Could not acquire auth config lock");
                app.login_error = Some("SYSTEM ERROR - Please try again".to_string());
            }
            
            app.username_input.reset();
            app.password_input.reset();
            app.focus = LoginFocus::Username;
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                LoginFocus::Username => LoginFocus::Password,
                LoginFocus::Password => LoginFocus::Username,
                _ => app.focus,
            }
        }
        _ => {
            app.login_error = None;
            match app.focus {
                LoginFocus::Username => {
                    app.username_input.handle_event(&Event::Key(key));
                }
                LoginFocus::Password => {
                    app.password_input.handle_event(&Event::Key(key));
                }
                _ => {}
            };
        }
    }
}

fn handle_authenticated_keys(
    app: &mut App,
    key: KeyEvent,
    directives: &DirectiveConfig,
    dictionary: &DirectiveDictionary,
    engine: Arc<TokioMutex<DirectiveEngine>>,
    config: &Config,
    config_manager: Arc<TokioMutex<ConfigManager>>,
    log_buffer: &LogBuffer,
    dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
) {
    let consumed = match app.auth_mode {
        AuthenticatedMode::DirectiveView => {
            handle_directive_view_keys(app, key, directives, dictionary, engine, config)
        }
        AuthenticatedMode::PluginMgmtView => {
            handle_plugin_mgmt_keys(app, key, config, config_manager)
        }
        AuthenticatedMode::DictionaryEditorView => {
            handle_dictionary_editor_keys(app, key, dictionary, dictionary_manager)
        }
        AuthenticatedMode::LogView => handle_log_view_keys(app, key),
        _ => false,
    };

    if consumed {
        return;
    }

    match key.code {
        KeyCode::Char('1') => app.auth_mode = AuthenticatedMode::PluginView,
        KeyCode::Char('2') => app.auth_mode = AuthenticatedMode::LogView,
        KeyCode::Char('3') => app.auth_mode = AuthenticatedMode::DirectiveView,
        KeyCode::Char('4') => app.auth_mode = AuthenticatedMode::PluginMgmtView,
        KeyCode::Char('5') => app.auth_mode = AuthenticatedMode::DictionaryEditorView,
        KeyCode::Char('s') => {
            if StdFs::create_dir_all("logs/snapshot").is_ok() {
                let logs_to_save = log_buffer.lock().unwrap();
                let twenty_four_hours = Duration::from_secs(24 * 60 * 60);
                let recent_logs: Vec<&str> = logs_to_save
                    .iter()
                    .filter(|(instant, _)| instant.elapsed() < twenty_four_hours)
                    .map(|(_, log_str)| log_str.as_str())
                    .collect();

                let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                let filename = format!("logs/snapshot/APOLLO_snapshot_{}.log", timestamp);
                let content = recent_logs.join("\n");

                match StdFs::write(&filename, &content) {
                    Ok(_) => {
                        app.status_message =
                            Some((format!("Snapshot saved to {}", filename), Instant::now()))
                    }
                    Err(e) => {
                        app.status_message =
                            Some((format!("Error saving snapshot: {}", e), Instant::now()))
                    }
                }
            } else {
                app.status_message =
                    Some(("Error creating snapshot directory.".to_string(), Instant::now()));
            }
        }
        KeyCode::Char('q') => {
            log::info!("[AUTH] User logged out");
            app.logout();
        }
        _ => {}
    }
}

fn handle_directive_view_keys(
    app: &mut App,
    key: KeyEvent,
    directives: &DirectiveConfig,
    dictionary: &DirectiveDictionary,
    engine: Arc<TokioMutex<DirectiveEngine>>,
    config: &Config,
) -> bool {
    if key.code == KeyCode::End {
        app.reset_wizard();
        return true;
    }

    if key.code == KeyCode::Backspace && app.directive_mode == DirectiveMode::Add {
        match app.directive_form_step {
            DirectiveFormStep::SelectStatus => {
                app.directive_form_step = DirectiveFormStep::SelectTarget;
                return true;
            }
            DirectiveFormStep::SelectActionTarget => {
                app.directive_form_step = DirectiveFormStep::SelectStatus;
                return true;
            }
            DirectiveFormStep::SelectCommand => {
                app.directive_form_step = DirectiveFormStep::SelectActionTarget;
                return true;
            }
            DirectiveFormStep::Confirm => {
                app.directive_form_step = DirectiveFormStep::SelectCommand;
                return true;
            }
            _ => {}
        }
    }

    if app.directive_mode == DirectiveMode::View
        || (app.directive_mode == DirectiveMode::Add
            && (app.directive_form_step == DirectiveFormStep::SelectTarget
                || app.directive_form_step == DirectiveFormStep::SelectActionTarget))
        || app.directive_mode == DirectiveMode::Remove
    {
        match key.code {
            KeyCode::Right => {
                if app.directive_form_step == DirectiveFormStep::SelectTarget {
                    app.directive_mode = match app.directive_mode {
                        DirectiveMode::View => DirectiveMode::Add,
                        DirectiveMode::Add => DirectiveMode::Remove,
                        DirectiveMode::Remove => DirectiveMode::View,
                    };
                    return true;
                }
            }
            KeyCode::Left => {
                if app.directive_form_step == DirectiveFormStep::SelectTarget {
                    app.directive_mode = match app.directive_mode {
                        DirectiveMode::View => DirectiveMode::Remove,
                        DirectiveMode::Add => DirectiveMode::View,
                        DirectiveMode::Remove => DirectiveMode::Add,
                    };
                    return true;
                }
            }
            _ => {}
        }
    }

    if app.directive_mode == DirectiveMode::Add {
        return match app.directive_form_step {
            DirectiveFormStep::SelectTarget | DirectiveFormStep::SelectActionTarget => {
                handle_list_nav_keys(app, key, config, directives, dictionary)
            }
            DirectiveFormStep::SelectStatus | DirectiveFormStep::SelectCommand => {
                handle_wip_list_nav_keys(app, key, dictionary)
            }
            DirectiveFormStep::Confirm => {
                if key.code == KeyCode::Insert {
                    let wip = app.wip_directive.clone();
                    let new_rule = Rule {
                        if_status_is: wip.status,
                        then_command_target: wip.command_target,
                        then_command_json: wip.command_json,
                    };
                    tokio::spawn(async move {
                        let mut engine_guard = engine.lock().await;
                        if let Err(e) =
                            engine_guard.add_and_save_rule(&wip.target, new_rule).await
                        {
                            log::error!("[CORE] Failed to save directive: {}", e);
                        }
                    });
                    app.status_message =
                        Some(("Directive saved successfully!".to_string(), Instant::now()));
                    app.reset_wizard();
                    return true;
                }
                false
            }
        };
    } else if app.directive_mode == DirectiveMode::View || app.directive_mode == DirectiveMode::Remove {
        return handle_list_nav_keys(app, key, config, directives, dictionary);
    }

    false
}

fn handle_plugin_mgmt_keys(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    config_manager: Arc<TokioMutex<ConfigManager>>,
) -> bool {
    match key.code {
        KeyCode::Right => {
            app.plugin_mgmt_mode = match app.plugin_mgmt_mode {
                PluginMgmtMode::View => PluginMgmtMode::AddPlugin,
                PluginMgmtMode::AddPlugin => PluginMgmtMode::AddSubsection,
                PluginMgmtMode::AddSubsection => PluginMgmtMode::AddEntity,
                PluginMgmtMode::AddEntity => PluginMgmtMode::Remove,
                PluginMgmtMode::Remove => PluginMgmtMode::View,
            };
            app.reset_plugin_mgmt_form();
            return true;
        }
        KeyCode::Left => {
            app.plugin_mgmt_mode = match app.plugin_mgmt_mode {
                PluginMgmtMode::View => PluginMgmtMode::Remove,
                PluginMgmtMode::AddPlugin => PluginMgmtMode::View,
                PluginMgmtMode::AddSubsection => PluginMgmtMode::AddPlugin,
                PluginMgmtMode::AddEntity => PluginMgmtMode::AddSubsection,
                PluginMgmtMode::Remove => PluginMgmtMode::AddEntity,
            };
            app.reset_plugin_mgmt_form();
            return true;
        }
        _ => {}
    }

    let consumed = match app.plugin_mgmt_mode {
        PluginMgmtMode::View | PluginMgmtMode::Remove => {
            let mut tree_items = Vec::new();
            views::plugin_mgmt::build_config_tree_items(&mut tree_items, config, &app.plugin_mgmt_expanded);
            let selected = app.plugin_mgmt_list_state.selected();
            match key.code {
                KeyCode::Up => { if let Some(s) = selected { app.plugin_mgmt_list_state.select(Some(s.saturating_sub(1))); } else if !tree_items.is_empty() { app.plugin_mgmt_list_state.select(Some(0)); } true },
                KeyCode::Down => { if let Some(s) = selected { if s < tree_items.len() - 1 { app.plugin_mgmt_list_state.select(Some(s + 1)); } } else if !tree_items.is_empty() { app.plugin_mgmt_list_state.select(Some(0)); } true },
                KeyCode::Tab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.insert(item.id.clone()); } } } true },
                KeyCode::BackTab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.remove(&item.id); } } } true },
                KeyCode::Delete => {
                    if app.plugin_mgmt_mode == PluginMgmtMode::Remove {
                        if let Some(s) = selected {
                            if let Some(item) = tree_items.get(s) {
                                app.wip_action = Some(WipAction::PluginComponentDeletion(item.id.clone()));
                            }
                        }
                    }
                    true
                }
                _ => false,
            }
        }
        PluginMgmtMode::AddPlugin => {
            match key.code {
                KeyCode::Tab => { app.focus = match app.focus { LoginFocus::FieldOne => LoginFocus::FieldTwo, _ => LoginFocus::FieldOne } },
                KeyCode::Enter => {
                    if !app.input_one.value().is_empty() && !app.input_two.value().is_empty() {
                        let id = app.input_one.value().to_string();
                        let auth_key = app.input_two.value().to_string();
                        tokio::spawn(async move {
                            let mut manager = config_manager.lock().await;
                            if let Err(e) = manager.add_plugin(id, auth_key).await { log::error!("[CORE] Failed to save new plugin: {}", e); }
                        });
                        app.status_message = Some(("New plugin saved.".to_string(), Instant::now()));
                        app.plugin_mgmt_mode = PluginMgmtMode::View;
                        app.reset_plugin_mgmt_form();
                    }
                }
                _ => { match app.focus { LoginFocus::FieldOne => { app.input_one.handle_event(&Event::Key(key)); }, LoginFocus::FieldTwo => { app.input_two.handle_event(&Event::Key(key)); }, _ => {} }; }
            }
            true
        }
        PluginMgmtMode::AddSubsection => {
            match app.plugin_mgmt_form_step {
                PluginMgmtFormStep::SelectParent => {
                    let mut tree_items = Vec::new();
                    views::plugin_mgmt::build_config_tree_items(&mut tree_items, config, &app.plugin_mgmt_expanded);
                    let selected = app.plugin_mgmt_list_state.selected();
                    match key.code {
                        KeyCode::Up => { if let Some(s) = selected { app.plugin_mgmt_list_state.select(Some(s.saturating_sub(1))); } },
                        KeyCode::Down => { if let Some(s) = selected { if s < tree_items.len() - 1 { app.plugin_mgmt_list_state.select(Some(s + 1)); } } else if !tree_items.is_empty() { app.plugin_mgmt_list_state.select(Some(0)); } },
                        KeyCode::Tab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.insert(item.id.clone()); } } } },
                        KeyCode::BackTab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.remove(&item.id); } } } },
                        KeyCode::Enter => {
                            if let Some(s) = selected {
                                if let Some(item) = tree_items.get(s) {
                                    if !item.id.contains('_') {
                                        app.wip_parent_id = item.id.clone();
                                        app.plugin_mgmt_form_step = PluginMgmtFormStep::EnterDetails;
                                        app.focus = LoginFocus::FieldOne;
                                    }
                                }
                            }
                        }
                        _ => return false,
                    }
                }
                PluginMgmtFormStep::EnterDetails => {
                    match key.code {
                        KeyCode::Backspace => { app.plugin_mgmt_form_step = PluginMgmtFormStep::SelectParent; },
                        KeyCode::Tab => { app.focus = match app.focus { LoginFocus::FieldOne => LoginFocus::FieldTwo, _ => LoginFocus::FieldOne } },
                        KeyCode::Enter => {
                            let parent_id = app.wip_parent_id.clone();
                            let sub_id = app.input_one.value().to_string();
                            let auth_key = app.input_two.value().to_string();
                            if !parent_id.is_empty() && !sub_id.is_empty() && !auth_key.is_empty() {
                                tokio::spawn(async move {
                                    let mut manager = config_manager.lock().await;
                                    if let Err(e) = manager.add_subsection(&parent_id, sub_id, auth_key).await { log::error!("[CORE] Failed to save new subsection: {}", e); }
                                });
                                app.status_message = Some(("New subsection saved.".to_string(), Instant::now()));
                                app.plugin_mgmt_mode = PluginMgmtMode::View;
                                app.reset_plugin_mgmt_form();
                            }
                        }
                        _ => { match app.focus { LoginFocus::FieldOne => { app.input_one.handle_event(&Event::Key(key)); }, LoginFocus::FieldTwo => { app.input_two.handle_event(&Event::Key(key)); }, _ => {} }; }
                    }
                }
            }
            true
        }
        PluginMgmtMode::AddEntity => {
            match app.plugin_mgmt_form_step {
                PluginMgmtFormStep::SelectParent => {
                    let mut tree_items = Vec::new();
                    views::plugin_mgmt::build_config_tree_items(&mut tree_items, config, &app.plugin_mgmt_expanded);
                    let selected = app.plugin_mgmt_list_state.selected();
                    match key.code {
                        KeyCode::Up => { if let Some(s) = selected { app.plugin_mgmt_list_state.select(Some(s.saturating_sub(1))); } },
                        KeyCode::Down => { if let Some(s) = selected { if s < tree_items.len() - 1 { app.plugin_mgmt_list_state.select(Some(s + 1)); } } else if !tree_items.is_empty() { app.plugin_mgmt_list_state.select(Some(0)); } },
                        KeyCode::Tab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.insert(item.id.clone()); } } } },
                        KeyCode::BackTab => { if let Some(s) = selected { if let Some(item) = tree_items.get(s) { if item.is_expandable { app.plugin_mgmt_expanded.remove(&item.id); } } } },
                        KeyCode::Enter => {
                            if let Some(s) = selected {
                                if let Some(item) = tree_items.get(s) {
                                    if item.id.matches('_').count() == 1 {
                                        app.wip_parent_id = item.id.clone();
                                        app.plugin_mgmt_form_step = PluginMgmtFormStep::EnterDetails;
                                        app.focus = LoginFocus::FieldOne;
                                    }
                                }
                            }
                        }
                        _ => return false,
                    }
                }
                PluginMgmtFormStep::EnterDetails => {
                    match key.code {
                        KeyCode::Backspace => { app.plugin_mgmt_form_step = PluginMgmtFormStep::SelectParent; },
                        KeyCode::Tab => { app.focus = match app.focus { LoginFocus::FieldOne => LoginFocus::FieldTwo, _ => LoginFocus::FieldOne } },
                        KeyCode::Enter => {
                            let parent_id = app.wip_parent_id.clone();
                            let entity_id = app.input_one.value().to_string();
                            let auth_key = app.input_two.value().to_string();
                            if !parent_id.is_empty() && !entity_id.is_empty() && !auth_key.is_empty() {
                                tokio::spawn(async move {
                                    let mut manager = config_manager.lock().await;
                                    if let Err(e) = manager.add_entity(&parent_id, entity_id, auth_key).await { log::error!("[CORE] Failed to save new entity: {}", e); }
                                });
                                app.status_message = Some(("New entity saved.".to_string(), Instant::now()));
                                app.plugin_mgmt_mode = PluginMgmtMode::View;
                                app.reset_plugin_mgmt_form();
                            }
                        }
                        _ => { match app.focus { LoginFocus::FieldOne => { app.input_one.handle_event(&Event::Key(key)); }, LoginFocus::FieldTwo => { app.input_two.handle_event(&Event::Key(key)); }, _ => {} }; }
                    }
                }
            }
            true
        }
    };
    
    consumed
}

fn handle_list_nav_keys(
    app: &mut App,
    key: KeyEvent,
    config: &Config,
    directives: &DirectiveConfig,
    dictionary: &DirectiveDictionary,
) -> bool {
    let mut tree_items = Vec::new();
    views::directives::build_tree_items(&mut tree_items, config, directives, &app.expanded_directives);
    let selected = app.directive_list_state.selected();

    match key.code {
        KeyCode::Up => {
            if let Some(s) = selected {
                app.directive_list_state.select(Some(s.saturating_sub(1)));
            } else if !tree_items.is_empty() {
                app.directive_list_state.select(Some(0));
            }
        }
        KeyCode::Down => {
            if let Some(s) = selected {
                if s < tree_items.len() - 1 {
                    app.directive_list_state.select(Some(s + 1));
                }
            } else if !tree_items.is_empty() {
                app.directive_list_state.select(Some(0));
            }
        }
        KeyCode::Tab => {
            if let Some(s) = selected {
                if let Some(item) = tree_items.get(s) {
                    if item.node_type != NodeType::Rule && item.node_type != NodeType::Entity {
                        app.expanded_directives.insert(item.id.clone());
                    }
                }
            }
        }
        KeyCode::BackTab => {
            if let Some(s) = selected {
                if let Some(item) = tree_items.get(s) {
                    if item.node_type != NodeType::Rule && item.node_type != NodeType::Entity {
                        app.expanded_directives.remove(&item.id);
                    }
                }
            }
        }
        KeyCode::Enter => {
            if app.directive_mode == DirectiveMode::Add {
                if let Some(s) = selected {
                    if let Some(item) = tree_items.get(s) {
                        match app.directive_form_step {
                            DirectiveFormStep::SelectTarget => {
                                app.wip_directive.target = item.id.clone();
                                let target_plugin_type =
                                    item.id.split('_').next().unwrap_or("");
                                app.wip_choices.clear();
                                if let Some(dict) =
                                    dictionary.plugin_dictionaries.get(target_plugin_type)
                                {
                                    for code in &dict.status_codes {
                                        app.wip_choices.push((code.clone(), code.clone()));
                                    }
                                }
                                if let Some(dict) = dictionary.plugin_dictionaries.get("generic") {
                                    for code in &dict.status_codes {
                                        if !app.wip_choices.iter().any(|(c, _)| c == code) {
                                            app.wip_choices.push((code.clone(), code.clone()));
                                        }
                                    }
                                }
                                app.wip_list_state
                                    .select(if app.wip_choices.is_empty() { None } else { Some(0) });
                                app.directive_form_step = DirectiveFormStep::SelectStatus;
                            }
                            DirectiveFormStep::SelectActionTarget => {
                                app.wip_directive.command_target = item.id.clone();
                                let target_plugin_type =
                                    item.id.split('_').next().unwrap_or("");
                                app.wip_choices.clear();
                                if let Some(dict) =
                                    dictionary.plugin_dictionaries.get(target_plugin_type)
                                {
                                    for (key, template) in &dict.command_templates {
                                        app.wip_choices
                                            .push((template.name.clone(), key.clone()));
                                    }
                                }
                                if let Some(dict) = dictionary.plugin_dictionaries.get("generic") {
                                    for (key, template) in &dict.command_templates {
                                        if !app.wip_choices.iter().any(|(_d, v)| v == key) {
                                            app.wip_choices
                                                .push((template.name.clone(), key.clone()));
                                        }
                                    }
                                }
                                app.wip_list_state
                                    .select(if app.wip_choices.is_empty() { None } else { Some(0) });
                                app.directive_form_step = DirectiveFormStep::SelectCommand;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        KeyCode::Delete => {
            if app.directive_mode == DirectiveMode::Remove {
                if let Some(s) = selected {
                    if let Some(item) = tree_items.get(s) {
                        if item.node_type == NodeType::Rule {
                            if let Some(rule) = &item.rule {
                                app.wip_action = Some(WipAction::DirectiveDeletion(
                                    item.parent_id.clone(),
                                    rule.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => return false,
    }
    true
}

fn handle_wip_list_nav_keys(
    app: &mut App,
    key: KeyEvent,
    dictionary: &DirectiveDictionary,
) -> bool {
    let selected = app.wip_list_state.selected();
    let choice_count = app.wip_choices.len();

    match key.code {
        KeyCode::Up => {
            if let Some(s) = selected {
                app.wip_list_state.select(Some(s.saturating_sub(1)));
            }
        }
        KeyCode::Down => {
            if let Some(s) = selected {
                if s < choice_count.saturating_sub(1) {
                    app.wip_list_state.select(Some(s + 1));
                }
            } else if choice_count > 0 {
                app.wip_list_state.select(Some(0));
            }
        }
        KeyCode::Enter => {
            if let Some(s) = selected {
                if let Some((name, value)) = app.wip_choices.get(s).cloned() {
                    if app.directive_form_step == DirectiveFormStep::SelectStatus {
                        app.wip_directive.status = value;
                        app.directive_form_step = DirectiveFormStep::SelectActionTarget;
                    } else if app.directive_form_step == DirectiveFormStep::SelectCommand {
                        app.wip_directive.command_name = name;
                        let target_plugin_type =
                            app.wip_directive.command_target.split('_').next().unwrap_or("");
                        let mut command_json = "{}".to_string();
                        if let Some(dict) = dictionary.plugin_dictionaries.get(target_plugin_type) {
                            if let Some(template) = dict.command_templates.get(&value) {
                                let template_str = serde_json::to_string(template).unwrap_or_default();
                                command_json = template_str;
                            }
                        }
                        if command_json == "{}" {
                            if let Some(dict) = dictionary.plugin_dictionaries.get("generic") {
                                if let Some(template) = dict.command_templates.get(&value) {
                                    let template_str = serde_json::to_string(template).unwrap_or_default();
                                    command_json = template_str;
                                }
                            }
                        }
                        app.wip_directive.command_json = command_json;
                        app.directive_form_step = DirectiveFormStep::Confirm;
                    }
                }
            }
        }
        _ => return false,
    }
    true
}

fn handle_delete_confirmation(app: &mut App, key: KeyEvent, engine: Arc<TokioMutex<DirectiveEngine>>) {
    if let Some(WipAction::DirectiveDeletion(target_id, rule)) = app.wip_action.clone() {
        match key.code {
            KeyCode::Enter => {
                if app.input_one.value() == "APOLLO-CONFIRM-DELETE" {
                    tokio::spawn(async move {
                        let mut engine_guard = engine.lock().await;
                        if let Err(e) = engine_guard.remove_and_save_rule(&target_id, &rule).await {
                            log::error!("[CORE] Failed to remove directive: {}", e);
                        }
                    });
                    app.status_message = Some(("Directive removed.".to_string(), Instant::now()));
                    app.reset_confirmation();
                } else {
                    app.status_message = Some(("Confirmation code incorrect. Action cancelled.".to_string(), Instant::now()));
                    app.reset_confirmation();
                }
            }
            KeyCode::Esc | KeyCode::End => {
                app.reset_confirmation();
            }
            _ => {
                app.input_one.handle_event(&Event::Key(key));
            }
        }
    }
}

// fn handle_plugin_delete_confirmation(
    // app: &mut App,
    // key: KeyEvent,
    // config_manager: Arc<TokioMutex<ConfigManager>>,
// ) {
    // if let Some(target_id) = app.wip_action.clone() {
        // if let WipAction::PluginComponentDeletion(id) = target_id {
            // match key.code {
                // KeyCode::Enter => {
                    // if app.input_one.value() == "APOLLO-CONFIRM-DELETE" {
                        // tokio::spawn(async move {
                            // let mut manager = config_manager.lock().await;
                            // let id_parts: Vec<&str> = id.split('_').collect();
                            // let result = match id_parts.len() {
                                // 1 => manager.remove_plugin(id_parts[0]).await,
                                // 2 => manager.remove_subsection(id_parts[0], id_parts[1]).await,
                                // 3 => manager.remove_entity(&format!("{}_{}", id_parts[0], id_parts[1]), id_parts[2]).await,
                                // _ => Ok(()),
                            // };
                            // if let Err(e) = result {
                                // log::error!("[CORE] Failed to remove component: {}", e);
                            // }
                        // });
                        // app.status_message = Some((format!("Component '{}' removed.", id), Instant::now()));
                        // app.reset_confirmation();
                    // } else {
                        // app.status_message = Some(("Confirmation code incorrect. Action cancelled.".to_string(), Instant::now()));
                        // app.reset_confirmation();
                    // }
                // }
                // KeyCode::Esc | KeyCode::End => {
                    // app.reset_confirmation();
                // }
                // _ => {
                    // app.input_one.handle_event(&Event::Key(key));
                // }
            // }
        // }
    // }
// }

fn handle_dictionary_editor_keys(
    app: &mut App,
    key: KeyEvent,
    dictionary: &DirectiveDictionary,
    dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
) -> bool {
    // If we are in an input form, handle that first
    if app.dict_editor_mode == DictionaryEditorMode::AddStatus {
        if let Some(plugin_id) = app.dict_selected_plugin.clone() {
            match key.code {
                KeyCode::Enter => {
                    let new_code = app.input_one.value().to_string();
                    if !new_code.is_empty() {
                        tokio::spawn(async move {
                            let mut manager = dictionary_manager.lock().await;
                            if let Err(e) = manager.add_status_code(&plugin_id, new_code).await {
                                log::error!("[DICT] Failed to add status code: {}", e);
                            }
                        });
                        app.status_message =
                            Some(("New status code saved.".to_string(), Instant::now()));
                        app.input_one.reset();
                        app.dict_editor_mode = DictionaryEditorMode::View;
                    }
                }
                KeyCode::End => {
                    app.input_one.reset();
                    app.dict_editor_mode = DictionaryEditorMode::View;
                }
                _ => {
                    app.input_one.handle_event(&Event::Key(key));
                }
            }
        } else {
            app.dict_editor_mode = DictionaryEditorMode::View;
        }
        return true;
    } else if app.dict_editor_mode == DictionaryEditorMode::AddCommand {
        if let Some(plugin_id) = app.dict_selected_plugin.clone() {
            match key.code {
                KeyCode::Enter => {
                    let name = app.input_one.value().to_string();
                    let key_val = app.input_two.value().to_string();
                    let priority = match app.wip_command_priority {
                        0 => "LOW",
                        1 => "MED",
                        _ => "HIGH",
                    }
                    .to_string();
                    let has_level = app.wip_command_has_level;
                    let level = if has_level { app.wip_command_level } else { 0 };

                    if !name.is_empty() && !key_val.is_empty() {
                        let template = CommandTemplate {
                            name,
                            priority,
                            has_level,
                            level,
                        };
                        tokio::spawn(async move {
                            let mut manager = dictionary_manager.lock().await;
                            if let Err(e) =
                                manager.add_command_template(&plugin_id, key_val, template).await
                            {
                                log::error!("[DICT] Failed to add command: {}", e);
                            }
                        });
                        app.status_message = Some(("New command saved.".to_string(), Instant::now()));
                        app.dict_editor_mode = DictionaryEditorMode::View;
                        app.reset_command_form();
                    }
                }
                KeyCode::End => {
                    app.dict_editor_mode = DictionaryEditorMode::View;
                    app.reset_command_form();
                }
                KeyCode::Tab => app.command_form_focus = app.command_form_focus.next(),
                KeyCode::BackTab => app.command_form_focus = app.command_form_focus.prev(),
                _ => {
                    let _ = match app.command_form_focus {
                        CommandFormFocus::Name => app.input_one.handle_event(&Event::Key(key)),
                        CommandFormFocus::Key => app.input_two.handle_event(&Event::Key(key)),
                        CommandFormFocus::Priority => {
                            if let KeyCode::Right = key.code {
                                app.wip_command_priority = (app.wip_command_priority + 1) % 3;
                            } else if let KeyCode::Left = key.code {
                                app.wip_command_priority = (app.wip_command_priority + 2) % 3;
                            }
                            None
                        }
                        CommandFormFocus::HasLevel => {
                            if let KeyCode::Char(' ') = key.code {
                                app.wip_command_has_level = !app.wip_command_has_level;
                            }
                            None
                        }
                        CommandFormFocus::Level => {
                            if app.wip_command_has_level {
                                if let KeyCode::Right = key.code {
                                    app.wip_command_level = (app.wip_command_level % 5) + 1;
                                } else if let KeyCode::Left = key.code {
                                    app.wip_command_level =
                                        if app.wip_command_level == 1 { 5 } else { app.wip_command_level - 1 };
                                }
                            }
                            None
                        }
                    };
                }
            }
        } else {
            app.dict_editor_mode = DictionaryEditorMode::View;
        }
        return true;
    }

    let mut plugin_keys: Vec<String> = dictionary.plugin_dictionaries.keys().cloned().collect();
    plugin_keys.sort();

    match key.code {
        KeyCode::Tab => {
            app.dict_active_column = match app.dict_active_column {
                DictionaryColumn::Plugins => {
                    if app.dict_status_list_state.selected().is_none() { app.dict_status_list_state.select(Some(0)); }
                    DictionaryColumn::StatusCodes
                },
                DictionaryColumn::StatusCodes => {
                    if app.dict_command_list_state.selected().is_none() { app.dict_command_list_state.select(Some(0)); }
                    DictionaryColumn::Commands
                },
                DictionaryColumn::Commands => DictionaryColumn::Plugins,
            };
        }
        KeyCode::BackTab => {
            app.dict_active_column = match app.dict_active_column {
                DictionaryColumn::Plugins => {
                    if app.dict_command_list_state.selected().is_none() { app.dict_command_list_state.select(Some(0)); }
                    DictionaryColumn::Commands
                },
                DictionaryColumn::StatusCodes => DictionaryColumn::Plugins,
                DictionaryColumn::Commands => {
                    if app.dict_status_list_state.selected().is_none() { app.dict_status_list_state.select(Some(0)); }
                    DictionaryColumn::StatusCodes
                },
            };
        }
        KeyCode::Down => match app.dict_active_column {
            DictionaryColumn::Plugins => {
                let count = plugin_keys.len();
                if count == 0 { return true; }
                let selected = app.dict_plugin_list_state.selected().unwrap_or(0);
                let next = if selected >= count.saturating_sub(1) { selected } else { selected + 1 };
                app.dict_plugin_list_state.select(Some(next));
            },
            DictionaryColumn::StatusCodes => {
                if let Some(plugin_key) = &app.dict_selected_plugin {
                    if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(plugin_key) {
                        let count = plugin_dict.status_codes.len();
                        if count == 0 { return true; }
                        let selected = app.dict_status_list_state.selected().unwrap_or(0);
                        let next = if selected >= count.saturating_sub(1) { selected } else { selected + 1 };
                        app.dict_status_list_state.select(Some(next));
                    }
                }
            },
            DictionaryColumn::Commands => {
                if let Some(plugin_key) = &app.dict_selected_plugin {
                    if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(plugin_key) {
                        let count = plugin_dict.command_templates.len();
                        if count == 0 { return true; }
                        let selected = app.dict_command_list_state.selected().unwrap_or(0);
                        let next = if selected >= count.saturating_sub(1) { selected } else { selected + 1 };
                        app.dict_command_list_state.select(Some(next));
                    }
                }
            },
        },
        KeyCode::Up => match app.dict_active_column {
            DictionaryColumn::Plugins => {
                let selected = app.dict_plugin_list_state.selected().unwrap_or(0);
                let prev = selected.saturating_sub(1);
                app.dict_plugin_list_state.select(Some(prev));
            },
            DictionaryColumn::StatusCodes => {
                let selected = app.dict_status_list_state.selected().unwrap_or(0);
                let prev = selected.saturating_sub(1);
                app.dict_status_list_state.select(Some(prev));
            },
            DictionaryColumn::Commands => {
                let selected = app.dict_command_list_state.selected().unwrap_or(0);
                let prev = selected.saturating_sub(1);
                app.dict_command_list_state.select(Some(prev));
            },
        },
        KeyCode::Insert => {
            app.dict_editor_mode = match app.dict_active_column {
                DictionaryColumn::StatusCodes => DictionaryEditorMode::AddStatus,
                DictionaryColumn::Commands => DictionaryEditorMode::AddCommand,
                _ => app.dict_editor_mode,
            };
            app.reset_command_form();
        }
        KeyCode::Delete => {
            if let Some(plugin_id) = app.dict_selected_plugin.clone() {
                match app.dict_active_column {
                    DictionaryColumn::StatusCodes => {
                        if let Some(selected_index) = app.dict_status_list_state.selected() {
                            if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(&plugin_id) {
                                if let Some(code) = plugin_dict.status_codes.get(selected_index) {
                                    app.wip_action = Some(WipAction::DictionaryItemDeletion(plugin_id, code.clone()));
                                }
                            }
                        }
                    }
                    DictionaryColumn::Commands => {
                        if let Some(selected_index) = app.dict_command_list_state.selected() {
                            if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(&plugin_id) {
                                let mut cmd_keys: Vec<_> = plugin_dict.command_templates.keys().collect();
                                cmd_keys.sort();
                                if let Some(key) = cmd_keys.get(selected_index) {
                                     app.wip_action = Some(WipAction::DictionaryItemDeletion(plugin_id, (*key).clone()));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            return true;
        }
        _ => return false,
    }

    true
}

// fn handle_dictionary_delete_confirmation(
    // app: &mut App,
    // key: KeyEvent,
    // dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
// ) {
    // if let Some((plugin_id, item)) = app.dict_wip_deletion.clone() {
        // match key.code {
            // KeyCode::Char('y') | KeyCode::Char('Y') => {
                // let item_clone_for_message = item.clone();
                // let active_column = app.dict_active_column;

                // tokio::spawn(async move {
                    // let mut manager = dictionary_manager.lock().await;
                    
                    // if active_column == DictionaryColumn::StatusCodes {
                        // if let Err(e) = manager.remove_status_code(&plugin_id, &item).await {
                            // log::error!("[DICT] Failed to remove status code: {}", e);
                        // }
                    // } else if active_column == DictionaryColumn::Commands {
                        // if let Err(e) = manager.remove_command_template(&plugin_id, &item).await {
                            // log::error!("[DICT] Failed to remove command template: {}", e);
                        // }
                    // }
                // });
                
                // app.status_message = Some((
                    // format!("'{}' removed from dictionary.", item_clone_for_message),
                    // Instant::now(),
                // ));
                // app.dict_wip_deletion = None;
                // Reset list selections
                // app.dict_status_list_state.select(None);
                // app.dict_command_list_state.select(None);
            // }
            // KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::End => {
                // app.dict_wip_deletion = None;
            // }
            // _ => {}
        // }
    // }
// }

fn handle_log_view_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Right => {
            app.log_viewer_mode = LogViewerMode::Browser;
            return true;
        }
        KeyCode::Left => {
            app.log_viewer_mode = LogViewerMode::Live;
            return true;
        }
        _ => {}
    }

    if app.log_viewer_mode == LogViewerMode::Browser {
        if !app.selected_log_file_content.is_empty() {
            let selected = app.selected_log_file_state.selected();
            let content_len = app.selected_log_file_content.len();
            match key.code {
                KeyCode::Up => {
                    if let Some(s) = selected {
                        app.selected_log_file_state
                            .select(Some(s.saturating_sub(1)));
                    } else if content_len > 0 {
                        app.selected_log_file_state.select(Some(0));
                    }
                }
                KeyCode::Down => {
                    if let Some(s) = selected {
                        if s < content_len.saturating_sub(1) {
                            app.selected_log_file_state.select(Some(s + 1));
                        }
                    } else if content_len > 0 {
                        app.selected_log_file_state.select(Some(0));
                    }
                }
                KeyCode::Backspace | KeyCode::Esc => {
                    app.selected_log_file_content.clear();
                    app.selected_log_file_state = ListState::default();
                }
                _ => return false,
            }
        } else {
            let selected = app.log_file_list_state.selected();
            let file_count = app.log_files.len();
            match key.code {
                KeyCode::Up => {
                    if let Some(s) = selected {
                        app.log_file_list_state
                            .select(Some(s.saturating_sub(1)));
                    } else if file_count > 0 {
                        app.log_file_list_state.select(Some(0));
                    }
                }
                KeyCode::Down => {
                    if let Some(s) = selected {
                        if s < file_count.saturating_sub(1) {
                            app.log_file_list_state.select(Some(s + 1));
                        }
                    } else if file_count > 0 {
                        app.log_file_list_state.select(Some(0));
                    }
                }
                KeyCode::Enter => {
                    if let Some(s) = selected {
                        if let Some(file_name) = app.log_files.get(s) {
                            if let Ok(lines) = read_log_file(file_name) {
                                if !lines.is_empty() {
                                    app.selected_log_file_state.select(Some(0));
                                }
                                app.selected_log_file_content = lines;
                            }
                        }
                    }
                }
                _ => return false,
            }
        }
    } else {
        return false;
    }
    true
}

fn get_log_files() -> io::Result<Vec<String>> {
    let mut files = StdFs::read_dir("./logs")?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().into_string().unwrap_or_default())
        .collect::<Vec<String>>();
    files.sort();
    files.reverse();
    Ok(files)
}

fn read_log_file(file_name: &str) -> io::Result<Vec<String>> {
    let file = StdFs::File::open(format!("./logs/{}", file_name))?;
    let reader = BufReader::new(file);
    reader.lines().collect()
}

fn handle_secure_confirmation(
    app: &mut App,
    key: KeyEvent,
    engine: Arc<TokioMutex<DirectiveEngine>>,
    config_manager: Arc<TokioMutex<ConfigManager>>,
    dictionary_manager: Arc<TokioMutex<DictionaryManager>>,
    security_codes: Arc<SecurityCodes>,
) {
    match key.code {
        KeyCode::Enter => {
            let mut expected_code = "";
            let default_code = "".to_string();
            if let Some(action) = &app.wip_action {
                expected_code = match action {
                    WipAction::DirectiveDeletion(_, _) => security_codes.get_code("DirectiveDeletion").unwrap_or(&default_code),
                    WipAction::PluginComponentDeletion(id) => {
                        match id.matches('_').count() {
                            0 => security_codes.get_code("PluginDeletion").unwrap_or(&default_code),
                            1 => security_codes.get_code("SubsectionDeletion").unwrap_or(&default_code),
                            _ => security_codes.get_code("EntityDeletion").unwrap_or(&default_code),
                        }
                    },
                    WipAction::DictionaryItemDeletion(_, _) => {
                        match app.dict_active_column {
                            DictionaryColumn::StatusCodes => security_codes.get_code("StatusCodeDeletion").unwrap_or(&default_code),
                            DictionaryColumn::Commands => security_codes.get_code("CommandCodeDeletion").unwrap_or(&default_code),
                            _ => ""
                        }
                    }
                };
            }

            if app.input_one.value() == expected_code {
                if let Some(action) = app.wip_action.clone() {
                    let active_column = app.dict_active_column;
                    tokio::spawn(async move {
                        match action {
                            WipAction::DirectiveDeletion(target_id, rule) => {
                                let mut engine_guard = engine.lock().await;
                                if let Err(e) = engine_guard.remove_and_save_rule(&target_id, &rule).await {
                                    log::error!("[CORE] Failed to remove directive: {}", e);
                                }
                            },
                            WipAction::PluginComponentDeletion(target_id) => {
                                let mut manager = config_manager.lock().await;
                                let id_parts: Vec<&str> = target_id.split('_').collect();
                                let result = match id_parts.len() {
                                    1 => manager.remove_plugin(id_parts[0]).await,
                                    2 => manager.remove_subsection(id_parts[0], id_parts[1]).await,
                                    3 => manager.remove_entity(&format!("{}_{}", id_parts[0], id_parts[1]), id_parts[2]).await,
                                    _ => Ok(()),
                                };
                                if let Err(e) = result { log::error!("[CORE] Failed to remove component: {}", e); }
                            },
                            WipAction::DictionaryItemDeletion(plugin_id, item) => {
                                let mut manager = dictionary_manager.lock().await;
                                if active_column == DictionaryColumn::StatusCodes {
                                    if let Err(e) = manager.remove_status_code(&plugin_id, &item).await {
                                        log::error!("[DICT] Failed to remove status code: {}", e);
                                    }
                                } else if active_column == DictionaryColumn::Commands {
                                    if let Err(e) = manager.remove_command_template(&plugin_id, &item).await {
                                        log::error!("[DICT] Failed to remove command template: {}", e);
                                    }
                                }
                            }
                        }
                    });
                    app.status_message = Some(("Action confirmed. Operation in progress.".to_string(), Instant::now()));
                }
                app.reset_confirmation();
            } else {
                app.status_message = Some(("Authentication code incorrect. Action cancelled.".to_string(), Instant::now()));
                app.reset_confirmation();
            }
        }
        KeyCode::Esc | KeyCode::End => {
            app.reset_confirmation();
        }
        _ => {
            app.input_one.handle_event(&Event::Key(key));
        }
    }
}

fn ui(
    f: &mut Frame,
    app: &mut App,
    log_items: &[ListItem],
    plugins: &[PluginState],
    directives: &DirectiveConfig,
    config: &Config,
    dictionary: &DirectiveDictionary,
) {
    match app.mode {
        AppMode::SplashScreen => splash::draw(f),
        AppMode::Login => login::draw(f, app),
        AppMode::Authenticated => {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.size());

            let title = Paragraph::new(" APOLLO Mainframe")
                .style(Style::default().add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, main_layout[0]);

            let titles = vec!["(1) Plugin Monitor", "(2) System Logs", "(3) System Directives", "(4) Plugin Management", "(5) Dictionary Editor"];
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL))
                .select(app.auth_mode as usize)
                .style(Style::default().fg(Color::Gray))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            f.render_widget(tabs, main_layout[1]);

            let content_area = main_layout[2];
            match app.auth_mode {
                AuthenticatedMode::PluginView => views::plugin_monitor::draw(f, app, content_area, plugins),
                AuthenticatedMode::LogView => views::log_viewer::draw(f, app, log_items, content_area),
                AuthenticatedMode::DirectiveView => views::directives::draw(f, app, content_area, directives, config),
                AuthenticatedMode::PluginMgmtView => views::plugin_mgmt::draw(f, app, content_area, config),
                AuthenticatedMode::DictionaryEditorView => views::dictionary_editor::draw(f, app, content_area, dictionary),
            }
            
            let footer = Paragraph::new("SYSTEM COMMANDS: (s) Save Logs | (q) Logout | (Esc) Quit")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, main_layout[3]);
            
            if let Some((msg, _)) = &app.status_message {
                let area = centered_rect(60, 20, f.size());
                let text = Paragraph::new(msg.as_str()).alignment(Alignment::Center);
                f.render_widget(Clear, area);
                f.render_widget(text.block(Block::default().title("System Message").borders(Borders::ALL)), area);
            }

            if app.wip_action.is_some() {
                let mut title = "Confirm Action".to_string();
                let mut lines = Vec::new();

                if let Some(WipAction::DirectiveDeletion(target, rule)) = &app.wip_action {
                    title = "Confirm Directive Deletion".to_string();
                    lines.push(Line::from(vec!["Delete rule ".into(), "'".into(), rule.if_status_is.clone().red(), "'".into()]));
                    lines.push(Line::from(vec!["from target ".into(), target.clone().yellow(), "?".into()]));
                } else if let Some(WipAction::PluginComponentDeletion(target)) = &app.wip_action {
                    title = "Confirm Component Deletion".to_string();
                    lines.push(Line::from(vec!["Delete component ".into(), target.clone().red(), "?".into()]));
                    lines.push(Line::from("This will delete the component and all its children.".gray()));
                } else if let Some(WipAction::DictionaryItemDeletion(plugin, item)) = &app.wip_action {
                    title = "Confirm Dictionary Deletion".to_string();
                    lines.push(Line::from(vec!["Delete item ".into(), item.clone().red(), "?".into()]));
                    lines.push(Line::from(vec!["from plugin group ".into(), plugin.clone().yellow(), "?".into()]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from("This action is irreversible.".underlined()));
                lines.push(Line::from("Please enter APOLLO Deletion Authentication Code"));

                let area = centered_rect(60, 50, f.size());
                let text = Paragraph::new(lines).alignment(Alignment::Center).wrap(Wrap { trim: true });
                let input = Paragraph::new(app.input_one.value()).block(Block::default().borders(Borders::ALL));

                let popup_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
                    .margin(1)
                    .split(area);

                f.render_widget(Clear, area);
                f.render_widget(Block::default().title(title).borders(Borders::ALL), area);
                f.render_widget(text, popup_chunks[0]);
                f.render_widget(input, popup_chunks[1]);
                f.set_cursor(
                    popup_chunks[1].x + app.input_one.cursor() as u16 + 1,
                    popup_chunks[1].y + 1,
                );
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

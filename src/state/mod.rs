// src/state/mod.rs

use crate::core::directives::Rule;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::time::Instant;
use tui_input::Input;

pub mod app_state;
pub mod config;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppMode {
    SplashScreen,
    Login,
    Authenticated,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoginFocus {
    Username,
    Password,
    FieldOne,
    FieldTwo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AuthenticatedMode {
    PluginView,
    LogView,
    DirectiveView,
    PluginMgmtView,
    DictionaryEditorView,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DirectiveMode {
    View,
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PluginMgmtMode {
    View,
    AddPlugin,
    AddSubsection,
    AddEntity,
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DirectiveFormStep {
    SelectTarget,
    SelectStatus,
    SelectActionTarget,
    SelectCommand,
    Confirm,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PluginMgmtFormStep {
    SelectParent,
    EnterDetails,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogViewerMode {
    Live,
    Browser,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DictionaryColumn {
    Plugins,
    StatusCodes,
    Commands,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DictionaryEditorMode {
    View,
    AddStatus,
    AddCommand,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandFormFocus {
    Name,
    Key,
    Priority,
    HasLevel,
    Level,
}

impl CommandFormFocus {
    pub fn next(&self) -> Self {
        match *self {
            Self::Name => Self::Key,
            Self::Key => Self::Priority,
            Self::Priority => Self::HasLevel,
            Self::HasLevel => Self::Level,
            Self::Level => Self::Name,
        }
    }
    pub fn prev(&self) -> Self {
        match *self {
            Self::Name => Self::Level,
            Self::Key => Self::Name,
            Self::Priority => Self::Key,
            Self::HasLevel => Self::Priority,
            Self::Level => Self::HasLevel,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WipDirective {
    pub target: String,
    pub status: String,
    pub command_target: String,
    pub command_name: String,
    pub command_json: String,
}


#[derive(Clone, Debug)]
pub enum WipAction {
    DirectiveDeletion(String, Rule),
    PluginComponentDeletion(String),
    DictionaryItemDeletion(String, String),
}

pub struct App {
    // ---Global State---
    pub mode: AppMode,
    pub auth_mode: AuthenticatedMode,

    // ---Form/Input State---
    pub focus: LoginFocus,
    pub username_input: Input,
    pub password_input: Input,
    pub login_error: Option<String>,
    pub input_one: Input,
    pub input_two: Input,

    // ---Authenticated View State ---
    pub log_list_state: ListState,
    pub directive_list_state: ListState,
    pub expanded_directives: HashSet<String>,
    pub directive_mode: DirectiveMode,
    pub directive_form_step: DirectiveFormStep,
    pub plugin_mgmt_list_state: ListState,
    pub plugin_mgmt_expanded: HashSet<String>,
    pub plugin_mgmt_mode: PluginMgmtMode,
    pub plugin_mgmt_form_step: PluginMgmtFormStep,
    pub log_viewer_mode: LogViewerMode,
    pub log_files: Vec<String>,
    pub log_file_list_state: ListState,
    pub selected_log_file_content: Vec<String>,
    pub selected_log_file_state: ListState,
    pub dict_plugin_list_state: ListState,
    pub dict_status_list_state: ListState,
    pub dict_command_list_state: ListState,
    pub dict_selected_plugin: Option<String>,
    pub dict_active_column: DictionaryColumn,
    pub dict_editor_mode: DictionaryEditorMode,
    pub command_form_focus: CommandFormFocus,

    // ---Wizard & Action State---
    pub wip_directive: WipDirective,
    pub wip_choices: Vec<(String, String)>,
    pub wip_list_state: ListState,
    pub status_message: Option<(String, Instant)>,
    pub wip_parent_id: String,
    pub wip_command_priority: u8,
    pub wip_command_has_level: bool,
    pub wip_command_level: u8,
    pub wip_action: Option<WipAction>, // Universal field for pending actions,.
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: AppMode::SplashScreen,
            auth_mode: AuthenticatedMode::PluginView,
            focus: LoginFocus::Username,
            username_input: Input::default(),
            password_input: Input::default(),
            login_error: None,
            input_one: Input::default(),
            input_two: Input::default(),
            log_list_state: ListState::default(),
            directive_list_state: ListState::default(),
            expanded_directives: HashSet::new(),
            directive_mode: DirectiveMode::View,
            directive_form_step: DirectiveFormStep::SelectTarget,
            plugin_mgmt_list_state: ListState::default(),
            plugin_mgmt_expanded: HashSet::new(),
            plugin_mgmt_mode: PluginMgmtMode::View,
            plugin_mgmt_form_step: PluginMgmtFormStep::SelectParent,
            log_viewer_mode: LogViewerMode::Live,
            log_files: Vec::new(),
            log_file_list_state: ListState::default(),
            selected_log_file_content: Vec::new(),
            selected_log_file_state: ListState::default(),
            dict_plugin_list_state: ListState::default(),
            dict_status_list_state: ListState::default(),
            dict_command_list_state: ListState::default(),
            dict_selected_plugin: None,
            dict_active_column: DictionaryColumn::Plugins,
            dict_editor_mode: DictionaryEditorMode::View,
            command_form_focus: CommandFormFocus::Name,
            wip_directive: WipDirective::default(),
            wip_choices: Vec::new(),
            wip_list_state: ListState::default(),
            status_message: None,
            wip_parent_id: String::new(),
            wip_command_priority: 0,
            wip_command_has_level: false,
            wip_command_level: 1,
            wip_action: None,
        }
    }

    pub fn reset_wizard(&mut self) {
        self.directive_form_step = DirectiveFormStep::SelectTarget;
        self.wip_directive = WipDirective::default();
        self.wip_choices.clear();
        self.wip_list_state = ListState::default();
        self.directive_mode = DirectiveMode::View;
    }
    
    pub fn reset_plugin_mgmt_form(&mut self) {
        self.plugin_mgmt_form_step = PluginMgmtFormStep::SelectParent;
        self.input_one.reset();
        self.input_two.reset();
        self.wip_parent_id.clear();
        self.focus = LoginFocus::Username;
    }
    
    pub fn reset_command_form(&mut self) {
        self.input_one.reset();
        self.input_two.reset();
        self.command_form_focus = CommandFormFocus::Name;
        self.wip_command_priority = 0;
        self.wip_command_has_level = false;
        self.wip_command_level = 1;
    }
    
    pub fn reset_confirmation(&mut self) {
        self.wip_action = None;
        self.input_one.reset();
    }

    pub fn logout(&mut self) {
        self.mode = AppMode::Login;
        self.username_input.reset();
        self.password_input.reset();
        self.focus = LoginFocus::Username;
    }
}
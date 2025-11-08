// src/ui/views/dictionary_editor.rs

use crate::core::dictionary::DirectiveDictionary;
use crate::state::{App, CommandFormFocus, DictionaryColumn, DictionaryEditorMode};
use crate::ui::centered_rect;
use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect, dictionary: &DirectiveDictionary) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let titles = vec!["View", "Add Status Code", "Add Command Template"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.dict_editor_mode as usize)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, main_chunks[0]);
    
    let content_area = main_chunks[1];
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(content_area);

    let focused_style = Style::default().bg(Color::DarkGray);
    let unfocused_style = Style::default();

    // Column 1: Plugin Groups.
    let mut plugin_keys: Vec<String> = dictionary.plugin_dictionaries.keys().cloned().collect();
    plugin_keys.sort();
    let plugin_items: Vec<ListItem> = plugin_keys
        .iter()
        .map(|key| ListItem::new(key.as_str()))
        .collect();
    
    let plugin_list = List::new(plugin_items)
        .block(Block::default().borders(Borders::ALL).title("Plugin Groups"))
        .highlight_style(if app.dict_active_column == DictionaryColumn::Plugins { focused_style } else { unfocused_style })
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(plugin_list, chunks[0], &mut app.dict_plugin_list_state);

    if let Some(selected_index) = app.dict_plugin_list_state.selected() {
        if let Some(key) = plugin_keys.get(selected_index) {
            if app.dict_selected_plugin.as_ref() != Some(key) {
                app.dict_status_list_state.select(None);
                app.dict_command_list_state.select(None);
            }
            app.dict_selected_plugin = Some(key.clone());
        }
    } else {
        app.dict_selected_plugin = None;
    }

    // Column 2: Status Codes..
    let status_items: Vec<ListItem> = if let Some(plugin_key) = &app.dict_selected_plugin {
        if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(plugin_key) {
            plugin_dict.status_codes.iter().map(|s| ListItem::new(s.as_str())).collect()
        } else { vec![] }
    } else { vec![] };

    let status_list = List::new(status_items)
        .block(Block::default().borders(Borders::ALL).title("Status Codes"))
        .highlight_style(if app.dict_active_column == DictionaryColumn::StatusCodes { focused_style } else { unfocused_style })
        .highlight_symbol(">> ");

    f.render_stateful_widget(status_list, chunks[1], &mut app.dict_status_list_state);

    //column 3: Command Templates.
    let command_items: Vec<ListItem> = if let Some(plugin_key) = &app.dict_selected_plugin {
        if let Some(plugin_dict) = dictionary.plugin_dictionaries.get(plugin_key) {
            plugin_dict.command_templates.iter().map(|(k, v)| {
                ListItem::new(format!("{}: {}", k, v.name))
            }).collect()
        } else { vec![] }
    } else { vec![] };

    let command_list = List::new(command_items)
        .block(Block::default().borders(Borders::ALL).title("Command Templates"))
        .highlight_style(if app.dict_active_column == DictionaryColumn::Commands { focused_style } else { unfocused_style })
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(command_list, chunks[2], &mut app.dict_command_list_state);

    if app.dict_editor_mode == DictionaryEditorMode::AddStatus {
        draw_add_status_popup(f, app);
    } else if app.dict_editor_mode == DictionaryEditorMode::AddCommand {
        draw_add_command_popup(f, app);
    }
}

fn draw_add_status_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 25, f.size());
    f.render_widget(Clear, area);

    let input = Paragraph::new(app.input_one.value())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("New Status Code (e.g., CriticalFailure)"));
    
    let text = if let Some(plugin) = &app.dict_selected_plugin {
        format!("Adding status code to plugin group: {}", plugin)
    } else {
        "Please select a plugin group first.".to_string()
    };
    let help_text = Paragraph::new(text).wrap(Wrap { trim: true });

    let popup_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .margin(1)
        .split(area);

    f.render_widget(Block::default().title("Add Status Code").borders(Borders::ALL), area);
    f.render_widget(help_text, popup_chunks[0]);
    f.render_widget(input, popup_chunks[1]);
    f.set_cursor(
        popup_chunks[1].x + app.input_one.cursor() as u16 + 1,
        popup_chunks[1].y + 1,
    );
}

fn draw_add_command_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 50, f.size());
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Key
            Constraint::Length(3), // Priority
            Constraint::Length(3), // Has level vheckbox
            Constraint::Length(3), // Level selector
            Constraint::Min(0),
        ])
        .margin(1)
        .split(area);
    
    let name_input = Paragraph::new(app.input_one.value()).style(if app.command_form_focus == CommandFormFocus::Name { Style::default().fg(Color::Yellow) } else { Style::default() })
        .block(Block::default().borders(Borders::ALL).title("Command Name (e.g., Core System Reboot)"));
    
    let key_input = Paragraph::new(app.input_two.value()).style(if app.command_form_focus == CommandFormFocus::Key { Style::default().fg(Color::Yellow) } else { Style::default() })
        .block(Block::default().borders(Borders::ALL).title("Command Key (e.g., reboot_core)"));

    let priorities = ["LOW", "MED", "HIGH"];
    let priority_text = format!("< {} >", priorities[app.wip_command_priority as usize]);
    let priority_selector = Paragraph::new(priority_text).alignment(Alignment::Center).style(if app.command_form_focus == CommandFormFocus::Priority { Style::default().fg(Color::Yellow) } else { Style::default() })
        .block(Block::default().borders(Borders::ALL).title("Priority"));

    let checkbox = if app.wip_command_has_level { "[x]" } else { "[ ]" };
    let has_level_check = Paragraph::new(format!("{} Has Levels?", checkbox)).style(if app.command_form_focus == CommandFormFocus::HasLevel { Style::default().fg(Color::Yellow) } else { Style::default() })
        .block(Block::default().borders(Borders::ALL));
    
    let level_text = if app.wip_command_has_level { format!("< {} >", app.wip_command_level) } else { "-".to_string() };
    let level_selector = Paragraph::new(level_text).alignment(Alignment::Center).style(if app.command_form_focus == CommandFormFocus::Level && app.wip_command_has_level { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) })
        .block(Block::default().borders(Borders::ALL).title("Level (1-5)"));

    f.render_widget(Block::default().title("Add Command Template").borders(Borders::ALL), area);
    f.render_widget(name_input, chunks[0]);
    f.render_widget(key_input, chunks[1]);
    f.render_widget(priority_selector, chunks[2]);
    f.render_widget(has_level_check, chunks[3]);
    f.render_widget(level_selector, chunks[4]);

    match app.command_form_focus {
        CommandFormFocus::Name => f.set_cursor(chunks[0].x + app.input_one.cursor() as u16 + 1, chunks[0].y + 1),
        CommandFormFocus::Key => f.set_cursor(chunks[1].x + app.input_two.cursor() as u16 + 1, chunks[1].y + 1),
        _ => {}
    }
}
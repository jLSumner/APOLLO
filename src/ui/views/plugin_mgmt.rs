// src/ui/views/plugin_mgmt.rs

use crate::state::{config::Config, App, LoginFocus, PluginMgmtFormStep, PluginMgmtMode};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ConfigTreeItem {
    pub id: String,
    pub display_text: String,
    pub is_expandable: bool,
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let titles = vec!["View Config", "Add Plugin", "Add Subsection", "Add Entity", "Remove"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.plugin_mgmt_mode as usize)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    let content_area = chunks[1];
    match app.plugin_mgmt_mode {
        PluginMgmtMode::View => draw_view_mode(f, app, content_area, config, "Plugin Configuration"),
        PluginMgmtMode::AddPlugin => draw_add_plugin_form(f, app, content_area),
        PluginMgmtMode::AddSubsection => draw_add_subsection_form(f, app, content_area, config),
        PluginMgmtMode::AddEntity => draw_add_entity_form(f, app, content_area, config),
        PluginMgmtMode::Remove => draw_view_mode(f, app, content_area, config, "Select Component to Remove (Press DELETE)"),
    }
}

fn draw_view_mode(f: &mut Frame, app: &mut App, area: Rect, config: &Config, title: &str) {
    let mut items = Vec::new();
    build_config_tree_items(&mut items, config, &app.plugin_mgmt_expanded);

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| ListItem::new(item.display_text.clone()))
        .collect();
    
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.plugin_mgmt_list_state);
}

fn draw_add_plugin_form(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);
    
    let instructions = Paragraph::new("Enter details for the new plugin group. Press Enter on the last field to submit.")
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(instructions, chunks[0]);

    let id_input = Paragraph::new(app.input_one.value())
        .style(match app.focus {
            LoginFocus::FieldOne => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("New Plugin Group ID (e.g., SEC, COMMS)"));
    f.render_widget(id_input, chunks[1]);
    
    let key_input = Paragraph::new(app.input_two.value())
        .style(match app.focus {
            LoginFocus::FieldTwo => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Auth Key for Plugin Group (e.g., -3000-)"));
    f.render_widget(key_input, chunks[2]);
    
    match app.focus {
        LoginFocus::FieldOne => {
            f.set_cursor(chunks[1].x + app.input_one.cursor() as u16 + 1, chunks[1].y + 1)
        }
        LoginFocus::FieldTwo => {
            f.set_cursor(chunks[2].x + app.input_two.cursor() as u16 + 1, chunks[2].y + 1)
        }
        _ => {}
    }
}

fn draw_add_subsection_form(f: &mut Frame, app: &mut App, area: Rect, config: &Config) {
    match app.plugin_mgmt_form_step {
        PluginMgmtFormStep::SelectParent => {
            draw_view_mode(f, app, area, config, "Step 1: Select Parent Plugin");
        }
        PluginMgmtFormStep::EnterDetails => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            
            let progress_text = vec![
                Line::from("Step 2: Enter Details".bold()),
                Line::from(""),
                Line::from("Parent Plugin:".gray()),
                Line::from(app.wip_parent_id.as_str()),
            ];
            let progress = Paragraph::new(progress_text)
                .block(Block::default().title("New Subsection").borders(Borders::ALL));
            f.render_widget(progress, chunks[0]);

            let form_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
                .split(chunks[1]);
            
            let id_input = Paragraph::new(app.input_one.value())
                .style(match app.focus {
                    LoginFocus::FieldOne => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .block(Block::default().borders(Borders::ALL).title("New Subsection ID (e.g., ENG)"));
            f.render_widget(id_input, form_chunks[0]);
            
            let key_input = Paragraph::new(app.input_two.value())
                .style(match app.focus {
                    LoginFocus::FieldTwo => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .block(Block::default().borders(Borders::ALL).title("Auth Key (e.g., -1200-)"));
            f.render_widget(key_input, form_chunks[1]);
            
            match app.focus {
                LoginFocus::FieldOne => { f.set_cursor(form_chunks[0].x + app.input_one.cursor() as u16 + 1, form_chunks[0].y + 1) }
                LoginFocus::FieldTwo => { f.set_cursor(form_chunks[1].x + app.input_two.cursor() as u16 + 1, form_chunks[1].y + 1) }
                _ => {}
            }
        }
    }
}

fn draw_add_entity_form(f: &mut Frame, app: &mut App, area: Rect, config: &Config) {
    match app.plugin_mgmt_form_step {
        PluginMgmtFormStep::SelectParent => {
            draw_view_mode(f, app, area, config, "Step 1: Select Parent Subsection");
        }
        PluginMgmtFormStep::EnterDetails => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            
            let progress_text = vec![
                Line::from("Step 2: Enter Details".bold()),
                Line::from(""),
                Line::from("Parent Subsection:".gray()),
                Line::from(app.wip_parent_id.as_str()),
            ];
            let progress = Paragraph::new(progress_text)
                .block(Block::default().title("New Entity").borders(Borders::ALL));
            f.render_widget(progress, chunks[0]);

            let form_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
                .split(chunks[1]);
            
            let id_input = Paragraph::new(app.input_one.value())
                .style(match app.focus {
                    LoginFocus::FieldOne => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .block(Block::default().borders(Borders::ALL).title("New Entity ID (e.g., DOOR_03)"));
            f.render_widget(id_input, form_chunks[0]);
            
            let key_input = Paragraph::new(app.input_two.value())
                .style(match app.focus {
                    LoginFocus::FieldTwo => Style::default().fg(Color::Yellow),
                    _ => Style::default(),
                })
                .block(Block::default().borders(Borders::ALL).title("Auth Key (e.g., -1103-)"));
            f.render_widget(key_input, form_chunks[1]);
            
            match app.focus {
                LoginFocus::FieldOne => { f.set_cursor(form_chunks[0].x + app.input_one.cursor() as u16 + 1, form_chunks[0].y + 1) }
                LoginFocus::FieldTwo => { f.set_cursor(form_chunks[1].x + app.input_two.cursor() as u16 + 1, form_chunks[1].y + 1) }
                _ => {}
            }
        }
    }
}

pub fn build_config_tree_items(
    items: &mut Vec<ConfigTreeItem>,
    config: &Config,
    expanded: &HashSet<String>,
) {
    for (plugin_id, plugin_data) in &config.plugins {
        let is_expanded = expanded.contains(plugin_id);
        let icon = if is_expanded { "▼" } else { "►" };
        items.push(ConfigTreeItem {
            id: plugin_id.clone(),
            display_text: format!("{} Plugin: {}", icon, plugin_id),
            is_expandable: true,
        });

        if is_expanded {
            for (sub_id, sub_data) in &plugin_data.subsections {
                let full_id = format!("{}_{}", plugin_id, sub_id);
                let is_sub_expanded = expanded.contains(&full_id);
                let sub_icon = if is_sub_expanded { "▼" } else { "►" };
                items.push(ConfigTreeItem {
                    id: full_id.clone(),
                    display_text: format!("   {} Subsection: {}", sub_icon, sub_id),
                    is_expandable: true,
                });

                if is_sub_expanded {
                    for entity_id in sub_data.entities.keys() {
                        items.push(ConfigTreeItem {
                            id: format!("{}_{}", full_id, entity_id),
                            display_text: format!("      └─ Entity: {}", entity_id),
                            is_expandable: false,
                        });
                    }
                }
            }
        }
    }
}
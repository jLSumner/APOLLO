// src/ui/views/directives.rs

use crate::core::directives::{DirectiveConfig, Rule};
use crate::state::{config::Config, App, DirectiveMode, DirectiveFormStep};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    Plugin,
    Subsection,
    Entity,
    Rule,
}

#[derive(Clone, Debug)]
pub struct TreeItem {
    pub id: String,
    pub display_text: String,
    pub node_type: NodeType,
    pub parent_id: String,
    pub rule: Option<Rule>,
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect, directives: &DirectiveConfig, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let titles = vec!["View Directives", "Add Directive", "Remove Directive"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.directive_mode as usize)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    let content_area = chunks[1];
    match app.directive_mode {
        DirectiveMode::View => draw_view_mode(f, app, content_area, directives, config, "System Directives"),
        DirectiveMode::Add => draw_add_mode(f, app, content_area, directives, config),
        DirectiveMode::Remove => draw_view_mode(f, app, content_area, directives, config, "Select a Directive to Remove (Press DELETE)"),
    }
}

fn draw_view_mode(f: &mut Frame, app: &mut App, area: Rect, directives: &DirectiveConfig, config: &Config, title: &str) {
    let mut items = Vec::new();
    build_tree_items(&mut items, config, directives, &app.expanded_directives);

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| ListItem::new(item.display_text.clone()))
        .collect();
    
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.directive_list_state);
}

fn draw_add_mode(f: &mut Frame, app: &mut App, area: Rect, directives: &DirectiveConfig, config: &Config) {
    match app.directive_form_step {
        DirectiveFormStep::SelectTarget | DirectiveFormStep::SelectActionTarget => {
            let title = if app.directive_form_step == DirectiveFormStep::SelectTarget {
                "Step 1 of 5: Select Trigger Target"
            } else {
                "Step 3 of 5: Select Action Target"
            };
            let list_block = Block::default().title(title).borders(Borders::ALL);
            let list_area = list_block.inner(area);
            f.render_widget(list_block, area);
            draw_view_mode(f, app, list_area, directives, config, title);
        }
        DirectiveFormStep::SelectStatus => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            let progress_text = vec![
                Line::from("Step 2 of 5: Select Trigger Status".bold()),
                Line::from(""),
                Line::from("Trigger Target:".gray()),
                Line::from(app.wip_directive.target.as_str()),
            ];
            let progress = Paragraph::new(progress_text)
                .block(Block::default().title("Directive Details").borders(Borders::ALL));
            f.render_widget(progress, chunks[0]);
            let choices: Vec<ListItem> = app.wip_choices.iter().map(|(d, _v)| ListItem::new(d.as_str())).collect();
            let list = List::new(choices)
                .block(Block::default().title("Available Status Codes").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray).bold())
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, chunks[1], &mut app.wip_list_state);
        }
        DirectiveFormStep::SelectCommand => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            let progress_text = vec![
                Line::from("Step 4 of 5: Select Action Command".bold()),
                Line::from(""),
                Line::from("Trigger Target:".gray()),
                Line::from(format!("  {}", app.wip_directive.target)),
                Line::from("Trigger Status:".gray()),
                Line::from(format!("  '{}'", app.wip_directive.status)),
                Line::from("Action Target:".gray()),
                Line::from(format!("  {}", app.wip_directive.command_target)),
            ];
            let progress = Paragraph::new(progress_text)
                .block(Block::default().title("Directive Details").borders(Borders::ALL));
            f.render_widget(progress, chunks[0]);
            let choices: Vec<ListItem> = app.wip_choices.iter().map(|(d, _v)| ListItem::new(d.as_str())).collect();
            let list = List::new(choices)
                .block(Block::default().title("Available Command Templates").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray).bold())
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, chunks[1], &mut app.wip_list_state);
        }
        DirectiveFormStep::Confirm => {
            let text = vec![
                Line::from("Step 5 of 5: Confirm Directive".bold()),
                Line::from(""),
                Line::from("Please review the new directive before saving:".underlined()),
                Line::from(""),
                Line::from("IF Target:".gray()),
                Line::from(format!("  {}", app.wip_directive.target)),
                Line::from("Has Status:".gray()),
                Line::from(format!("  '{}'", app.wip_directive.status)),
                Line::from("THEN Target:".gray()),
                Line::from(format!("  {}", app.wip_directive.command_target)),
                Line::from("With Command:".gray()),
                Line::from(format!("  {} ({})", app.wip_directive.command_name, app.wip_directive.command_json)),
                Line::from(""),
                Line::from(""),
                Line::from("Press INSERT to save, END to cancel, or Backspace to go back.".yellow()),
            ];
            let paragraph = Paragraph::new(text)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Confirm New Directive").borders(Borders::ALL));
            
            f.render_widget(paragraph, area);
        }
    }
}

pub fn build_tree_items(
    items: &mut Vec<TreeItem>,
    config: &Config,
    directives: &DirectiveConfig,
    expanded: &HashSet<String>,
) {
    for (plugin_id, plugin_data) in &config.plugins {
        let is_expanded = expanded.contains(plugin_id);
        let icon = if is_expanded { "▼" } else { "►" };
        items.push(TreeItem {
            id: plugin_id.clone(),
            display_text: format!("{} Plugin: {}", icon, plugin_id),
            node_type: NodeType::Plugin,
            parent_id: "".to_string(),
            rule: None,
        });

        if let Some(plugin_directives) = directives.plugins.get(plugin_id) {
            for rule in &plugin_directives.directives {
                items.push(TreeItem {
                    id: format!("{}-rule-{}", plugin_id, rule.if_status_is),
                    display_text: format!("   └─ IF '{}' THEN Target: {} -> {}", rule.if_status_is, rule.then_command_target, rule.then_command_json),
                    node_type: NodeType::Rule,
                    parent_id: plugin_id.clone(),
                    rule: Some(rule.clone()),
                });
            }
        }

        if is_expanded {
            for (sub_id, sub_data) in &plugin_data.subsections {
                let full_id = format!("{}_{}", plugin_id, sub_id);
                let is_sub_expanded = expanded.contains(&full_id);
                let sub_icon = if is_sub_expanded { "▼" } else { "►" };
                items.push(TreeItem {
                    id: full_id.clone(),
                    display_text: format!("   {} Subsection: {}", sub_icon, sub_id),
                    node_type: NodeType::Subsection,
                    parent_id: plugin_id.clone(),
                    rule: None,
                });

                if let Some(plugin_directives) = directives.plugins.get(plugin_id) {
                    if let Some(sub_directives) = plugin_directives.subsections.get(sub_id) {
                        for rule in &sub_directives.directives {
                             items.push(TreeItem {
                                id: format!("{}-rule-{}", full_id, rule.if_status_is),
                                display_text: format!("      └─ IF '{}' THEN Target: {} -> {}", rule.if_status_is, rule.then_command_target, rule.then_command_json),
                                node_type: NodeType::Rule,
                                parent_id: full_id.clone(),
                                rule: Some(rule.clone()),
                            });
                        }
                    }
                }

                if is_sub_expanded {
                    for (entity_id, _entity_data) in &sub_data.entities {
                        let full_entity_id = format!("{}_{}", full_id, entity_id);
                        items.push(TreeItem {
                            id: full_entity_id.clone(),
                            display_text: format!("      └─ Entity: {}", entity_id),
                            node_type: NodeType::Entity,
                            parent_id: full_id.clone(),
                            rule: None,
                        });
                        
                        if let Some(plugin_directives) = directives.plugins.get(plugin_id) {
                            if let Some(sub_directives) = plugin_directives.subsections.get(sub_id) {
                                if let Some(ent_directives) = sub_directives.entities.get(entity_id) {
                                     for rule in &ent_directives.directives {
                                        items.push(TreeItem {
                                            id: format!("{}-rule-{}", full_entity_id, rule.if_status_is),
                                            display_text: format!("         └─ IF '{}' THEN Target: {} -> {}", rule.if_status_is, rule.then_command_target, rule.then_command_json),
                                            node_type: NodeType::Rule,
                                            parent_id: full_entity_id.clone(),
                                            rule: Some(rule.clone()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
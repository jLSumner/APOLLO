// src/ui/views/log_viewer.rs

use crate::state::{App, LogViewerMode};
use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame, app: &mut App, log_items: &[ListItem], area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let titles = vec!["(TAB) Live View", "(TAB) Log Browser"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(app.log_viewer_mode as usize)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    let content_area = chunks[1];
    match app.log_viewer_mode {
        LogViewerMode::Live => draw_live_logs(f, app, content_area, log_items),
        LogViewerMode::Browser => draw_log_browser(f, app, content_area),
    }
}

fn draw_live_logs(f: &mut Frame, app: &mut App, area: Rect, log_items: &[ListItem]) {
    app.log_list_state.select(Some(log_items.len().saturating_sub(1)));

    let logs_list = List::new(log_items.to_vec())
        .block(Block::default().borders(Borders::ALL).title("Live System Logs"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(logs_list, area, &mut app.log_list_state);
}

fn draw_log_browser(f: &mut Frame, app: &mut App, area: Rect) {
    if !app.selected_log_file_content.is_empty() {

        let content_items: Vec<ListItem> = app.selected_log_file_content.iter()
            .map(|line| ListItem::new(line.clone()))
            .collect();
        
        let list = List::new(content_items)
            .block(Block::default().borders(Borders::ALL).title("Log File Content (Press Backspace to return)"))
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, area, &mut app.selected_log_file_state);

    } else {
        let file_items: Vec<ListItem> = app.log_files.iter()
            .map(|file_name| ListItem::new(file_name.clone()))
            .collect();

        let list = List::new(file_items)
            .block(Block::default().borders(Borders::ALL).title("Available Log Files"))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut app.log_file_list_state);
    }
}
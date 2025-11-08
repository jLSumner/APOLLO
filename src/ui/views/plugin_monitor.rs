// src/ui/views/plugin_monitor.rs

use crate::state::app_state::PluginState;
use crate::state::App;
use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame, _app: &mut App, area: Rect, plugins: &[PluginState]) {
    let items: Vec<ListItem> = plugins
        .iter()
        .map(|p| {
            let line = format!("ID: {} | Status: {} | Seen: {:.0?}s ago", p.entity_id, p.status, p.last_seen.elapsed());
            ListItem::new(line)
        })
        .collect();

    let plugins_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Active Plugins"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_widget(plugins_list, area);
}
// src/ui/login.rs

use crate::state::{App, LoginFocus};
use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame, app: &App) {
    let login_art = r#"
    █████╗ ██████╗  ██████╗ ██╗     ██╗      ██████╗     ███████╗██╗   ██╗███████╗████████╗███████╗███╗   ███╗███████╗
   ██╔══██╗██╔══██╗██╔═══██╗██║     ██║     ██╔═══██╗    ██╔════╝╚██╗ ██╔╝██╔════╝╚══██╔══╝██╔════╝████╗ ████║██╔════╝
   ███████║██████╔╝██║   ██║██║     ██║     ██║   ██║    ███████╗ ╚████╔╝ ███████╗   ██║   █████╗  ██╔████╔██║███████╗
   ██╔══██║██╔═══╝ ██║   ██║██║     ██║     ██║   ██║    ╚════██║  ╚██╔╝  ╚════██║   ██║   ██╔══╝  ██║╚██╔╝██║╚════██║
   ██║  ██║██║     ╚██████╔╝███████╗███████╗╚██████╔╝    ███████║   ██║   ███████║   ██║   ███████╗██║ ╚═╝ ██║███████║
   ╚═╝  ╚═╝╚═╝      ╚═════╝ ╚══════╝╚══════╝ ╚═════╝     ╚══════╝   ╚═╝   ╚══════╝   ╚═╝   ╚══════╝╚═╝     ╚═╝╚══════╝
                                                     
                                     Building Tomorrow, Today
"#;
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(10),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Percentage(20),
        ])
        .split(f.size());

    let art_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(70), Constraint::Percentage(15)])
        .split(chunks[1])[1];

    let field_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(30), Constraint::Percentage(35)]);
    
    let username_area = field_layout.split(chunks[3])[1];
    let password_area = field_layout.split(chunks[4])[1];
    
    let art = Paragraph::new(login_art)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    f.render_widget(art, art_area);

    let username_p = Paragraph::new(app.username_input.value())
        .style(match app.focus {
            LoginFocus::Username => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Username"));
    f.render_widget(username_p, username_area);

    let password_p = Paragraph::new("*".repeat(app.password_input.value().len()))
        .style(match app.focus {
            LoginFocus::Password => Style::default().fg(Color::Yellow),
            _ => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title("Password"));
    f.render_widget(password_p, password_area);

    if let Some(error) = &app.login_error {
        let error_p = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(error_p, chunks[5]);
    }

    match app.focus {
        LoginFocus::Username => {
            f.set_cursor(username_area.x + app.username_input.cursor() as u16 + 1, username_area.y + 1)
        }
        LoginFocus::Password => {
            f.set_cursor(password_area.x + app.password_input.cursor() as u16 + 1, password_area.y + 1)
        }
        _ => {}
    }
}
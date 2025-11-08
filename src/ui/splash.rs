// src/ui/splash.rs

use ratatui::{prelude::*, widgets::*};

pub fn draw(f: &mut Frame) {
    let splash_text = r#"
    █████╗ ██████╗  ██████╗ ██╗     ██╗      ██████╗     ███████╗██╗   ██╗███████╗████████╗███████╗███╗   ███╗███████╗
   ██╔══██╗██╔══██╗██╔═══██╗██║     ██║     ██╔═══██╗    ██╔════╝╚██╗ ██╔╝██╔════╝╚══██╔══╝██╔════╝████╗ ████║██╔════╝
   ███████║██████╔╝██║   ██║██║     ██║     ██║   ██║    ███████╗ ╚████╔╝ ███████╗   ██║   █████╗  ██╔████╔██║███████╗
   ██╔══██║██╔═══╝ ██║   ██║██║     ██║     ██║   ██║    ╚════██║  ╚██╔╝  ╚════██║   ██║   ██╔══╝  ██║╚██╔╝██║╚════██║
   ██║  ██║██║     ╚██████╔╝███████╗███████╗╚██████╔╝    ███████║   ██║   ███████║   ██║   ███████╗██║ ╚═╝ ██║███████║
   ╚═╝  ╚═╝╚═╝      ╚═════╝ ╚══════╝╚══════╝ ╚═════╝     ╚══════╝   ╚═╝   ╚══════╝   ╚═╝   ╚══════╝╚═╝     ╚═╝╚══════╝
                                                     
                                        SYSTEM BOOTING ...
"#;

    let text = Paragraph::new(splash_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));
    
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(f.size());

    f.render_widget(text, vertical_chunks[1]);
}
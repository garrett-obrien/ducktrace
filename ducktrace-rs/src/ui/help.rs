use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::centered_rect;

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(60, 80, f.area());

    // Clear the background
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(vec![
            Span::styled("DuckTrace", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" - Interactive Chart Viewer"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ←/→    ", Style::default().fg(Color::Green)),
            Span::raw("Switch between tabs"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓    ", Style::default().fg(Color::Green)),
            Span::raw("Scroll/select within tab"),
        ]),
        Line::from(vec![
            Span::styled("  Scroll ", Style::default().fg(Color::Green)),
            Span::raw("Scroll query or change selection"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Actions", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  x/Enter", Style::default().fg(Color::Green)),
            Span::raw(" Explain selected point (drill-down)"),
        ]),
        Line::from(vec![
            Span::styled("  c      ", Style::default().fg(Color::Green)),
            Span::raw("Clear data file"),
        ]),
        Line::from(vec![
            Span::styled("  ?      ", Style::default().fg(Color::Green)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  q      ", Style::default().fg(Color::Green)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Tabs", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Home   ", Style::default().fg(Color::Yellow)),
            Span::raw("Welcome screen and status"),
        ]),
        Line::from(vec![
            Span::styled("  Query  ", Style::default().fg(Color::Yellow)),
            Span::raw("View formatted SQL query"),
        ]),
        Line::from(vec![
            Span::styled("  Mask   ", Style::default().fg(Color::Yellow)),
            Span::raw("View column mappings (X/Y)"),
        ]),
        Line::from(vec![
            Span::styled("  Data   ", Style::default().fg(Color::Yellow)),
            Span::raw("Browse raw data rows"),
        ]),
        Line::from(vec![
            Span::styled("  Chart  ", Style::default().fg(Color::Yellow)),
            Span::raw("Visualize data (line/bar/scatter)"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

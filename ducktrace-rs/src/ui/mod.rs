pub mod tabs;
pub mod query;
pub mod mask;
pub mod data;
pub mod chart;
pub mod help;
pub mod explain;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Helper to create a centered rect as a percentage of the given area
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

use crate::app::{App, Tab};

/// Main render function that draws the entire UI
pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title bar
            Constraint::Length(2),  // Tabs
            Constraint::Min(5),     // Content
            Constraint::Length(1),  // Status bar
        ])
        .split(f.area());

    // Store layout areas for mouse hit testing
    app.layout.tabs_area = chunks[1];
    app.layout.content_area = chunks[2];

    // Title bar
    render_title(f, chunks[0], app);

    // Tabs
    tabs::render_tabs(f, chunks[1], app.active_tab);

    // Content area
    if let Some(ref data) = app.data {
        match app.active_tab {
            Tab::Query => query::render_query(f, chunks[2], data, app.scroll_offset),
            Tab::Mask => mask::render_mask(f, chunks[2], data),
            Tab::Data => {
                self::data::render_data(f, chunks[2], data, app.selected_point);
                app.layout.data_table_area = chunks[2];
            }
            Tab::Chart => {
                let chart_area = chart::render_chart(f, chunks[2], data, app.selected_point);
                app.layout.chart_area = chart_area;
            }
        }
    } else {
        render_waiting(f, chunks[2], app.frame);
    }

    // Status bar
    render_status_bar(f, chunks[3], app);

    // Explain overlay (on top of content)
    if app.show_explain {
        explain::render_explain(f, app);
    }

    // Help overlay (on top of everything)
    if app.show_help {
        help::render_help(f);
    }
}

fn render_title(f: &mut Frame, area: Rect, app: &App) {
    let title = if let Some(ref data) = app.data {
        format!("🦆 DuckTrace: {}", data.title)
    } else {
        "🦆 DuckTrace".to_string()
    };

    let paragraph = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_waiting(f: &mut Frame, area: Rect, frame: u32) {
    // Animated duck frames
    let duck_frames = [
        r#"
    __
  >(o )___
   ( ._> /
    `---'
        "#,
        r#"
     __
   >(o )___
    ( ._> /
     `---'
        "#,
        r#"
      __
    >(o )___
     ( ._> /
      `---'
        "#,
        r#"
     __
   >(o )___
    ( ._> /
     `---'
        "#,
    ];

    let frame_idx = (frame / 10 % duck_frames.len() as u32) as usize;
    let duck = duck_frames[frame_idx];

    let dots = ".".repeat(((frame / 5) % 4) as usize);
    let text = format!(
        "{}\n\n  Waiting for data{}\n\n  Watching: ~/.claude/ducktrace/current.json\n\n  Press ? for help, q to quit",
        duck, dots
    );

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Waiting for Chart Data ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status = if let Some(ref data) = app.data {
        if let Some(ref status) = data.status {
            format!(" {} | ", status)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let help_hint = "←→: tabs | ↑↓: select | x: explain | c: clear | ?: help | q: quit";

    let status_line = format!("{}{}", status, help_hint);

    let paragraph = Paragraph::new(status_line)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

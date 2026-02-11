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

    // Title bar
    render_title(f, chunks[0], app);

    // Tabs
    tabs::render_tabs(f, chunks[1], app.active_tab);

    // Content area
    match app.active_tab {
        Tab::Home => render_home(f, chunks[2], app),
        Tab::Query => {
            if let Some(ref data) = app.data {
                query::render_query(f, chunks[2], data, app.scroll_offset);
            } else {
                render_no_data(f, chunks[2]);
            }
        }
        Tab::Mask => {
            if let Some(ref data) = app.data {
                mask::render_mask(f, chunks[2], data);
            } else {
                render_no_data(f, chunks[2]);
            }
        }
        Tab::Data => {
            if let Some(ref data) = app.data {
                self::data::render_data(f, chunks[2], data, app.selected_point);
            } else {
                render_no_data(f, chunks[2]);
            }
        }
        Tab::Chart => {
            if let Some(ref data) = app.data {
                chart::render_chart(f, chunks[2], data, app.selected_point);
            } else {
                render_no_data(f, chunks[2]);
            }
        }
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

// Block-character ASCII art for "DuckTrace" with | delimiters between letters.
// Each segment between pipes gets its own color from the palette.
const DUCKTRACE_BANNER: &[&str] = &[
    " ██████╗ |██╗   ██╗| ██████╗|██╗  ██╗|████████╗|██████╗ | █████╗ | ██████╗|███████╗",
    " ██╔══██╗|██║   ██║|██╔════╝|██║ ██╔╝|╚══██╔══╝|██╔══██╗|██╔══██╗|██╔════╝|██╔════╝",
    " ██║  ██║|██║   ██║|██║     |█████╔╝ |   ██║   |██████╔╝|███████║|██║     |█████╗  ",
    " ██║  ██║|██║   ██║|██║     |██╔═██╗ |   ██║   |██╔══██╗|██╔══██║|██║     |██╔══╝  ",
    " ██████╔╝|╚██████╔╝|╚██████╗|██║  ██╗|   ██║   |██║  ██║|██║  ██║|╚██████╗|███████╗",
    " ╚═════╝ | ╚═════╝ | ╚═════╝|╚═╝  ╚═╝|   ╚═╝   |╚═╝  ╚═╝|╚═╝  ╚═╝| ╚═════╝|╚══════╝",
];

// Yellow-to-cyan gradient palette (one color per letter: D U C K T R A C E)
const BANNER_COLORS: &[(u8, u8, u8)] = &[
    (255, 255, 50),  // D — bright yellow
    (220, 245, 60),  // U — yellow-lime
    (180, 235, 80),  // C — lime
    (130, 220, 110), // K — yellow-green
    (80, 210, 150),  // T — green-teal
    (50, 200, 180),  // R — teal
    (40, 190, 210),  // A — teal-cyan
    (30, 180, 235),  // C — light cyan
    (0, 170, 255),   // E — cyan-blue
];

fn render_home(f: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    // Add a blank line for top padding
    lines.push(Line::from(""));

    // Render the banner with per-letter colors
    for banner_line in DUCKTRACE_BANNER {
        let segments: Vec<&str> = banner_line.split('|').collect();
        let spans: Vec<Span> = segments
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                let (r, g, b) = BANNER_COLORS[i % BANNER_COLORS.len()];
                Span::styled(
                    *seg,
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        lines.push(Line::from(spans));
    }

    // Tagline
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Interactive charts with data lineage from MotherDuck queries.",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::styled(
        "Select any data point and drill down into the underlying rows.",
        Style::default().fg(Color::White),
    ));

    // Getting Started
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Getting Started:",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::styled(
        "  1. Open a split terminal pane and run this TUI",
        Style::default().fg(Color::Gray),
    ));
    lines.push(Line::styled(
        "  2. In Claude Code, run /ducktrace to generate a chart",
        Style::default().fg(Color::Gray),
    ));
    lines.push(Line::styled(
        "  3. The chart appears here automatically",
        Style::default().fg(Color::Gray),
    ));

    // Quick Keys
    let key_style = Style::default().fg(Color::Green);
    let desc_style = Style::default().fg(Color::Gray);
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Quick Keys:  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("←→", key_style),
        Span::styled(" switch tabs  ", desc_style),
        Span::styled("↑↓", key_style),
        Span::styled(" select  ", desc_style),
        Span::styled("x", key_style),
        Span::styled(" drill-down  ", desc_style),
        Span::styled("?", key_style),
        Span::styled(" full help", desc_style),
    ]));

    // Status — depends on whether data is loaded
    lines.push(Line::from(""));
    if let Some(ref data) = app.data {
        lines.push(Line::styled(
            format!("✓ Data loaded: {}", data.title),
            Style::default().fg(Color::Green),
        ));
    } else {
        let dots = ".".repeat(((app.frame / 5) % 4) as usize);
        lines.push(Line::styled(
            format!("Waiting for data{}", dots),
            Style::default().fg(Color::Yellow),
        ));
    }
    lines.push(Line::styled(
        "Watching: ~/.claude/ducktrace/current.json",
        Style::default().fg(Color::DarkGray),
    ));

    // GitHub link
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Contributions welcome!",
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::styled(
        "github.com/garrett-obrien/ducktrace",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
    ));

    let (border_color, title) = if app.data.is_some() {
        (Color::Green, " Home ")
    } else {
        (Color::Yellow, " Home ")
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_no_data(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("No data loaded — use /ducktrace in Claude Code to generate a chart.")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::DarkGray))
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

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

fn render_banner_lines(lines: &mut Vec<Line>) {
    lines.push(Line::from(""));
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
}

fn format_history_timestamp(ts: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let d = UNIX_EPOCH + Duration::from_millis(ts);
    // Calculate month/day/hour/min from duration since epoch
    let secs = d.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // Simple date math (no timezone, UTC)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Calculate month and day from days since epoch (1970-01-01)
    let mut y = 1970i64;
    let mut remaining_days = days as i64;

    loop {
        let days_in_year = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let is_leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days = [31, if is_leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0u32;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining_days < md {
            month = i as u32 + 1;
            break;
        }
        remaining_days -= md;
    }
    let day = remaining_days + 1;

    format!("{:02}/{:02} {:02}:{:02}", month, day, hours, minutes)
}

fn render_home(f: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    render_banner_lines(&mut lines);

    if app.history.is_empty() {
        // No history — show original splash screen
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Interactive charts with data lineage from MotherDuck queries.",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            "Select any data point and drill down into the underlying rows.",
            Style::default().fg(Color::White),
        ));

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

        lines.push(Line::from(""));
        let dots = ".".repeat(((app.frame / 5) % 4) as usize);
        lines.push(Line::styled(
            format!("Waiting for data{}", dots),
            Style::default().fg(Color::Yellow),
        ));
        lines.push(Line::styled(
            "Watching: ~/.claude/ducktrace/current.json",
            Style::default().fg(Color::DarkGray),
        ));

        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Contributions welcome!",
            Style::default().fg(Color::DarkGray),
        ));
        lines.push(Line::styled(
            "github.com/garrett-obrien/ducktrace",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
        ));
    } else {
        // History exists — show data selector
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Recent Analyses:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

        for (i, entry) in app.history.iter().enumerate() {
            let is_selected = i == app.history_selected;
            let prefix = if is_selected { " \u{25b8} " } else { "   " };
            let ts = format_history_timestamp(entry.timestamp);
            let row_info = format!("{} rows", entry.row_count);

            let style = if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            // Truncate title to keep lines reasonable
            let max_title = 40;
            let title = if entry.title.len() > max_title {
                format!("{}...", &entry.title[..max_title - 3])
            } else {
                entry.title.clone()
            };

            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(title, style),
                Span::styled(format!("  {}  ", ts), Style::default().fg(Color::DarkGray)),
                Span::styled(row_info, Style::default().fg(Color::DarkGray)),
            ]);
            lines.push(line);
        }

        // Key hints
        let key_style = Style::default().fg(Color::Green);
        let desc_style = Style::default().fg(Color::DarkGray);
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" \u{2191}\u{2193}", key_style),
            Span::styled(": select  ", desc_style),
            Span::styled("Enter", key_style),
            Span::styled(": load  ", desc_style),
            Span::styled("d", key_style),
            Span::styled(": delete  ", desc_style),
            Span::styled("?", key_style),
            Span::styled(": help", desc_style),
        ]));

        // Status
        lines.push(Line::from(""));
        if let Some(ref data) = app.data {
            lines.push(Line::styled(
                format!("\u{2713} Data loaded: {}", data.title),
                Style::default().fg(Color::Green),
            ));
        } else {
            let dots = ".".repeat(((app.frame / 5) % 4) as usize);
            lines.push(Line::styled(
                format!("Waiting for data{}", dots),
                Style::default().fg(Color::Yellow),
            ));
        }
    }

    let (border_color, title) = if app.data.is_some() {
        (Color::Green, " Home ")
    } else if !app.history.is_empty() {
        (Color::Cyan, " Home ")
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

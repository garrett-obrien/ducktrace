use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use crate::app::App;
use crate::data::{format_value, value_to_string, ExplainData};
use super::centered_rect;

/// Render the explain overlay panel
pub fn render_explain(f: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, f.area());

    // Clear the background
    f.render_widget(Clear, area);

    // Render based on state
    if app.explain_loading {
        render_loading(f, area, app.frame);
    } else if let Some(ref error) = app.explain_error {
        render_error(f, area, error);
    } else if let Some(ref explain_data) = app.explain_data {
        render_data(f, area, explain_data, app);
    } else {
        render_loading(f, area, app.frame);
    }
}

fn render_loading(f: &mut Frame, area: Rect, frame: u32) {
    let dots = ".".repeat(((frame / 5) % 4) as usize);
    let text = format!(
        "\n\n\n  Loading drill-down data{}\n\n  Querying MotherDuck...",
        dots
    );

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Explain ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_error(f: &mut Frame, area: Rect, error: &str) {
    let text = format!("\n\n  Error:\n\n  {}\n\n  Press Esc to close", error);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Explain - Error ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_data(f: &mut Frame, area: Rect, explain_data: &ExplainData, app: &App) {
    // Split area: title/info at top, table in middle, help at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title/info
            Constraint::Min(5),    // Table
            Constraint::Length(1), // Help hint
        ])
        .margin(1)
        .split(area);

    // Render outer border
    let outer_block = Block::default()
        .title(format!(" {} ", explain_data.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    f.render_widget(outer_block, area);

    // Info line
    let total_info = if let Some(total) = explain_data.total_count {
        format!(
            "Showing {} of {} source rows",
            explain_data.rows.len(),
            total
        )
    } else {
        format!("{} source rows", explain_data.rows.len())
    };

    let info = Paragraph::new(total_info)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(info, chunks[0]);

    // Table
    if explain_data.rows.is_empty() {
        let empty = Paragraph::new("No source data found")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(empty, chunks[1]);
    } else {
        render_table(f, chunks[1], explain_data, app.explain_scroll);
    }

    // Help hint
    let help = Paragraph::new("↑↓ scroll | PgUp/PgDn page | Esc close")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);
}

fn render_table(f: &mut Frame, area: Rect, explain_data: &ExplainData, scroll: usize) {
    // Calculate column widths based on content
    let col_count = explain_data.columns.len();
    if col_count == 0 {
        return;
    }

    // Calculate available width per column
    let available_width = area.width.saturating_sub(2); // Account for borders
    let col_width = (available_width as usize / col_count).max(8);

    // Build header
    let header_cells: Vec<Cell> = explain_data
        .columns
        .iter()
        .map(|col| {
            Cell::from(truncate_for_width(col, col_width))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    // Build rows with scroll offset
    let visible_height = area.height.saturating_sub(3) as usize; // Account for header and borders
    let start_idx = scroll;
    let end_idx = (scroll + visible_height).min(explain_data.rows.len());

    let rows: Vec<Row> = explain_data.rows[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .map(|(col_idx, val)| {
                    let text = value_to_string(val);
                    let formatted = if col_idx < explain_data.columns.len() {
                        // Try to format numeric values based on column name
                        if let Some(num) = val.as_f64() {
                            format_value(num, &explain_data.columns[col_idx])
                        } else {
                            truncate_for_width(&text, col_width)
                        }
                    } else {
                        truncate_for_width(&text, col_width)
                    };

                    Cell::from(formatted).style(Style::default().fg(Color::White))
                })
                .collect();

            // Highlight selected row
            let style = if i == 0 && scroll == start_idx {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // Calculate widths constraint
    let widths: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Length(col_width as u16))
        .collect();

    let table = Table::new(rows, &widths)
        .header(header)
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(table, area);
}

fn truncate_for_width(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        s.chars().take(max_width).collect()
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}


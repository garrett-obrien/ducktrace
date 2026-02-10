use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::data::ChartData;

pub fn render_mask(f: &mut Frame, area: Rect, data: &ChartData) {
    let header_cells = ["Column", "Role", "Sample Value"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let x_idx = data.get_x_index();
    let y_idx = data.get_y_index();

    let rows: Vec<Row> = data
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let role = if i == x_idx {
                "X (Label)"
            } else if i == y_idx {
                "Y (Value)"
            } else {
                "-"
            };

            let sample = data
                .rows
                .first()
                .and_then(|row| row.get(i))
                .map(crate::data::value_to_string)
                .unwrap_or_else(|| "-".to_string());

            let style = if i == x_idx || i == y_idx {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            Row::new(vec![
                Cell::from(col.clone()).style(style),
                Cell::from(role).style(style),
                Cell::from(sample).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(40),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Column Mapping ({} columns) ", data.columns.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);

    // Show mapping summary at bottom
    let summary = format!(
        " X: {} → Y: {} ",
        data.x_field, data.y_field
    );
    let summary_area = Rect::new(
        area.x + 2,
        area.y + area.height - 1,
        summary.len() as u16,
        1,
    );
    let summary_widget = ratatui::widgets::Paragraph::new(summary)
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(summary_widget, summary_area);
}

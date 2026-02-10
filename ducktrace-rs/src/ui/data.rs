use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use crate::data::{format_value, truncate_string, value_to_string, ChartData};

pub fn render_data(f: &mut Frame, area: Rect, data: &ChartData, selected: usize) {
    let header_cells = data
        .columns
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let style = if i == data.get_x_index() || i == data.get_y_index() {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            };
            Cell::from(h.clone()).style(style)
        });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let y_idx = data.get_y_index();
    let y_field = &data.y_field;

    let rows: Vec<Row> = data
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .map(|(col_idx, val)| {
                    let display = if col_idx == y_idx {
                        if let Some(n) = val.as_f64() {
                            format_value(n, y_field)
                        } else if let Some(s) = val.as_str() {
                            if let Ok(n) = s.parse::<f64>() {
                                format_value(n, y_field)
                            } else {
                                value_to_string(val)
                            }
                        } else {
                            value_to_string(val)
                        }
                    } else {
                        let s = value_to_string(val);
                        truncate_string(&s, 30)
                    };

                    let style = if row_idx == selected {
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    } else if col_idx == data.get_x_index() {
                        Style::default().fg(Color::Cyan)
                    } else if col_idx == y_idx {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    Cell::from(display).style(style)
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    // Calculate column widths based on content
    let num_cols = data.columns.len();
    let widths: Vec<Constraint> = if num_cols == 0 {
        vec![]
    } else {
        vec![Constraint::Percentage((100 / num_cols) as u16); num_cols]
    };

    let mut title = format!(" Data ({} rows) ", data.rows.len());
    if let Some(truncated) = data.truncated_from {
        title = format!(" Data ({} rows, truncated from {}) ", data.rows.len(), truncated);
    }

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // Use stateful rendering for scroll support
    let mut state = TableState::default();
    state.select(Some(selected));

    f.render_stateful_widget(table, area, &mut state);

    // Show row indicator
    if !data.rows.is_empty() {
        let indicator = format!(" Row {}/{} ", selected + 1, data.rows.len());
        let indicator_area = Rect::new(
            area.x + area.width - indicator.len() as u16 - 2,
            area.y,
            indicator.len() as u16 + 1,
            1,
        );
        let indicator_widget =
            ratatui::widgets::Paragraph::new(indicator).style(Style::default().fg(Color::DarkGray));
        f.render_widget(indicator_widget, indicator_area);
    }
}

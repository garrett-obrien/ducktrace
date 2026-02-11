use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{Axis, Bar, BarChart, BarGroup, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};

use crate::data::{format_number, format_value, truncate_string, ChartData, ChartType};

/// Check if rows are in reverse chronological order (first x > last x)
fn is_reverse_sorted(data: &ChartData) -> bool {
    if data.rows.len() < 2 {
        return false;
    }
    let first_x = data.get_x_value(&data.rows[0]);
    let last_x = data.get_x_value(&data.rows[data.rows.len() - 1]);
    first_x > last_x
}

pub fn render_chart(f: &mut Frame, area: Rect, data: &ChartData, selected: usize) {
    let chart_type = data.infer_chart_type();

    // Split area for chart and selection info
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    let chart_area = chunks[0];
    let info_area = chunks[1];

    match chart_type {
        ChartType::Bar => render_bar_chart(f, chart_area, data, selected),
        ChartType::Line => render_line_chart(f, chart_area, data, selected, GraphType::Line),
        ChartType::Scatter => render_line_chart(f, chart_area, data, selected, GraphType::Scatter),
    }

    // Render selection info
    render_selection_info(f, info_area, data, selected);
}

fn render_bar_chart(f: &mut Frame, area: Rect, data: &ChartData, selected: usize) {
    if data.rows.is_empty() {
        render_empty(f, area);
        return;
    }

    let reversed = is_reverse_sorted(data);
    let len = data.rows.len();
    let max_y = data.max_y();
    let scale = if max_y > 0.0 { 100.0 / max_y } else { 1.0 };

    // Build bars in chronological order (reverse if data is DESC)
    let indices: Vec<usize> = if reversed {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };

    let bars: Vec<Bar> = indices
        .iter()
        .map(|&i| {
            let row = &data.rows[i];
            let label = data.get_x_value(row);
            let value = data.get_y_value(row);
            let scaled_value = (value * scale) as u64;

            let is_selected = i == selected;
            let style = if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Cyan)
            };

            Bar::default()
                .value(scaled_value)
                .label(Line::from(truncate_string(&label, 8)))
                .style(style)
                .value_style(if is_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                })
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(format!(" {} (Bar) ", data.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(5)
        .bar_gap(1)
        .max(100);

    f.render_widget(bar_chart, area);
}

fn render_line_chart(
    f: &mut Frame,
    area: Rect,
    data: &ChartData,
    selected: usize,
    graph_type: GraphType,
) {
    if data.rows.is_empty() {
        render_empty(f, area);
        return;
    }

    let reversed = is_reverse_sorted(data);
    let len = data.rows.len();

    // Build indices in chronological order
    let indices: Vec<usize> = if reversed {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };

    let points: Vec<(f64, f64)> = indices
        .iter()
        .enumerate()
        .map(|(chart_pos, &row_idx)| (chart_pos as f64, data.get_y_value(&data.rows[row_idx])))
        .collect();

    let min_y = data.min_y();
    let max_y = data.max_y();
    let y_range = max_y - min_y;
    let y_padding = y_range * 0.1;

    let y_bounds = [
        (min_y - y_padding).max(0.0),
        max_y + y_padding,
    ];

    let x_bounds = [0.0, (len - 1).max(1) as f64];

    // Main dataset
    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(graph_type)
        .style(Style::default().fg(Color::Cyan))
        .data(&points);

    // Selected point marker — map data index to chart position
    let selected_chart_pos = if reversed {
        len - 1 - selected
    } else {
        selected
    };
    let selected_point = vec![(selected_chart_pos as f64, data.get_y_value(&data.rows[selected]))];
    let selected_dataset = Dataset::default()
        .marker(Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::default().fg(Color::Yellow))
        .data(&selected_point);

    // X-axis labels (in chronological order)
    let first = &data.rows[*indices.first().unwrap()];
    let last = &data.rows[*indices.last().unwrap()];
    let x_labels: Vec<Span> = if len <= 5 {
        indices
            .iter()
            .map(|&i| Span::raw(truncate_string(&data.get_x_value(&data.rows[i]), 10)))
            .collect()
    } else {
        let mid = &data.rows[indices[len / 2]];
        vec![
            Span::raw(truncate_string(&data.get_x_value(first), 10)),
            Span::raw(truncate_string(&data.get_x_value(mid), 10)),
            Span::raw(truncate_string(&data.get_x_value(last), 10)),
        ]
    };

    // Y-axis labels
    let y_labels = vec![
        Span::raw(format_number(y_bounds[0])),
        Span::raw(format_number((y_bounds[0] + y_bounds[1]) / 2.0)),
        Span::raw(format_number(y_bounds[1])),
    ];

    let chart_type_name = match graph_type {
        GraphType::Line => "Line",
        GraphType::Scatter => "Scatter",
        _ => "Chart",
    };

    let chart = Chart::new(vec![dataset, selected_dataset])
        .block(
            Block::default()
                .title(format!(" {} ({}) ", data.title, chart_type_name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .x_axis(
            Axis::default()
                .title(data.x_field.clone())
                .style(Style::default().fg(Color::Gray))
                .bounds(x_bounds)
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title(data.y_field.clone())
                .style(Style::default().fg(Color::Gray))
                .bounds(y_bounds)
                .labels(y_labels),
        );

    f.render_widget(chart, area);
}

fn render_selection_info(f: &mut Frame, area: Rect, data: &ChartData, selected: usize) {
    if data.rows.is_empty() {
        return;
    }

    let row = &data.rows[selected];
    let x_val = data.get_x_value(row);
    let y_val = data.get_y_value(row);
    let y_formatted = format_value(y_val, &data.y_field);

    let info = format!(
        "◆ Point {}/{}: {} = {} → {} = {}",
        selected + 1,
        data.rows.len(),
        data.x_field,
        x_val,
        data.y_field,
        y_formatted
    );

    let paragraph = Paragraph::new(info)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn render_empty(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("No data to display")
        .block(
            Block::default()
                .title(" Chart ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

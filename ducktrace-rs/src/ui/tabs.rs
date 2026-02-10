use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs as RatatuiTabs},
};

use crate::app::Tab;

pub fn render_tabs(f: &mut Frame, area: Rect, active_tab: Tab) {
    let titles = vec!["1:Query", "2:Mask", "3:Data", "4:Chart"];

    let tabs = RatatuiTabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(active_tab as usize)
        .divider(symbols::DOT);

    f.render_widget(tabs, area);
}

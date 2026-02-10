use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use log::{debug, info};
use ratatui::prelude::Rect;

use crate::data::{ChartData, ExplainData};
use crate::ui::query::get_query_line_count;
use crate::watcher::{get_data_path, load_data};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Query = 0,
    Mask = 1,
    Data = 2,
    Chart = 3,
}

impl Tab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Query,
            1 => Tab::Mask,
            2 => Tab::Data,
            3 => Tab::Chart,
            _ => Tab::Query,
        }
    }

    pub fn next(&self) -> Self {
        Tab::from_index((*self as usize + 1) % 4)
    }

    pub fn prev(&self) -> Self {
        Tab::from_index((*self as usize + 3) % 4)
    }
}

/// Layout areas for mouse hit testing
#[derive(Default, Clone)]
pub struct LayoutAreas {
    pub tabs_area: Rect,
    pub content_area: Rect,
    pub chart_area: Rect,
    pub data_table_area: Rect,
}

pub struct App {
    pub data: Option<ChartData>,
    pub active_tab: Tab,
    pub scroll_offset: usize,
    pub selected_point: usize,
    pub show_help: bool,
    pub running: bool,
    pub frame: u32,
    pub layout: LayoutAreas,
    // Explain mode state
    pub show_explain: bool,
    pub explain_data: Option<ExplainData>,
    pub explain_loading: bool,
    pub explain_error: Option<String>,
    pub explain_scroll: usize,
    /// Pending drill-down query to execute (polled by main loop)
    pending_drill_down_query: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            data: None,
            active_tab: Tab::Query,
            scroll_offset: 0,
            selected_point: 0,
            show_help: false,
            running: true,
            frame: 0,
            layout: LayoutAreas::default(),
            show_explain: false,
            explain_data: None,
            explain_loading: false,
            explain_error: None,
            explain_scroll: 0,
            pending_drill_down_query: None,
        }
    }

    pub fn on_data_update(&mut self, data: ChartData) {
        self.selected_point = 0;
        self.scroll_offset = 0;
        self.data = Some(data);
    }

    pub fn refresh_data(&mut self) {
        let path = get_data_path();
        if let Ok(data) = load_data(&path) {
            self.on_data_update(data);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Any key closes help
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Handle explain overlay
        if self.show_explain {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.close_explain();
                }
                KeyCode::Up => {
                    self.explain_scroll = self.explain_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    if let Some(ref explain_data) = self.explain_data {
                        let max_scroll = explain_data.rows.len().saturating_sub(1);
                        self.explain_scroll = (self.explain_scroll + 1).min(max_scroll);
                    }
                }
                KeyCode::PageUp => {
                    self.explain_scroll = self.explain_scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    if let Some(ref explain_data) = self.explain_data {
                        let max_scroll = explain_data.rows.len().saturating_sub(1);
                        self.explain_scroll = (self.explain_scroll + 10).min(max_scroll);
                    }
                }
                KeyCode::Home => {
                    self.explain_scroll = 0;
                }
                KeyCode::End => {
                    if let Some(ref explain_data) = self.explain_data {
                        self.explain_scroll = explain_data.rows.len().saturating_sub(1);
                    }
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('r') => self.refresh_data(),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Left => self.active_tab = self.active_tab.prev(),
            KeyCode::Right => self.active_tab = self.active_tab.next(),
            KeyCode::Char(c) if ('1'..='4').contains(&c) => {
                self.active_tab = Tab::from_index((c as u8 - b'1') as usize);
            }
            // Explain selected point
            KeyCode::Char('x') | KeyCode::Enter => {
                if matches!(self.active_tab, Tab::Chart | Tab::Data) {
                    self.trigger_explain();
                }
            }
            KeyCode::Up => self.handle_up(),
            KeyCode::Down => self.handle_down(),
            KeyCode::Home => self.handle_home(),
            KeyCode::End => self.handle_end(),
            KeyCode::PageUp => self.handle_page_up(),
            KeyCode::PageDown => self.handle_page_down(),
            _ => {}
        }
    }

    /// Trigger explain mode for the currently selected data point
    fn trigger_explain(&mut self) {
        info!("trigger_explain called for point {}", self.selected_point);

        let Some(ref data) = self.data else {
            info!("No data available");
            return;
        };

        if data.rows.is_empty() || self.selected_point >= data.rows.len() {
            info!("Invalid selection: rows={}, selected={}", data.rows.len(), self.selected_point);
            return;
        }

        // Get selected point values
        let row = &data.rows[self.selected_point];
        let x_value = row
            .get(data.get_x_index())
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let y_value = row
            .get(data.get_y_index())
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        debug!("Selected x={:?}, y={:?}", x_value, y_value);
        debug!("drill_down config: {:?}", data.drill_down);
        debug!("database: {:?}", data.database);

        // Check if we have a drill-down query template
        let drill_down_query = if let Some(ref drill_down) = data.drill_down {
            // Substitute placeholders in the template
            let mut query = drill_down.query_template.clone();

            // Replace {{database}} placeholder if database is configured
            if let Some(ref db) = data.database {
                query = query.replace("{{database}}", db);
            }

            // Replace {{x}} placeholder (template controls quoting)
            let x_str = match &x_value {
                serde_json::Value::String(s) => s.replace('\'', "''"),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Null => "NULL".to_string(),
                _ => x_value.to_string().trim_matches('"').to_string(),
            };
            query = query.replace("{{x}}", &x_str);

            // Replace {{y}} placeholder (template controls quoting)
            let y_str = match &y_value {
                serde_json::Value::String(s) => s.replace('\'', "''"),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Null => "NULL".to_string(),
                _ => y_value.to_string().trim_matches('"').to_string(),
            };
            query = query.replace("{{y}}", &y_str);

            // Replace any custom param mappings (template controls quoting)
            for (placeholder, field_name) in &drill_down.param_mapping {
                if let Some(col_idx) = data.columns.iter().position(|c| c == field_name) {
                    if let Some(val) = row.get(col_idx) {
                        let val_str = match val {
                            serde_json::Value::String(s) => s.replace('\'', "''"),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Null => "NULL".to_string(),
                            _ => val.to_string().trim_matches('"').to_string(),
                        };
                        query = query.replace(&format!("{{{{{}}}}}", placeholder), &val_str);
                    }
                }
            }

            info!("Final drill-down query: {}", query);
            query
        } else {
            // No drill-down template - show error
            info!("No drill-down template configured");
            self.show_explain = true;
            self.explain_error = Some(
                "No drill-down query configured. Claude can provide drill-down \
                 metadata when generating charts."
                    .to_string(),
            );
            return;
        };

        // Set up UI state for loading
        self.show_explain = true;
        self.explain_loading = true;
        self.explain_error = None;
        self.explain_data = None;
        self.explain_scroll = 0;

        // Queue the query for execution by main loop
        self.pending_drill_down_query = Some(drill_down_query);
    }

    /// Take pending drill-down query (called by main loop)
    pub fn take_pending_drill_down(&mut self) -> Option<String> {
        self.pending_drill_down_query.take()
    }

    /// Handle successful drill-down result
    pub fn on_drill_down_success(&mut self, data: ExplainData) {
        self.explain_data = Some(data);
        self.explain_loading = false;
        self.explain_error = None;
    }

    /// Handle drill-down error
    pub fn on_drill_down_error(&mut self, error: String) {
        self.explain_error = Some(error);
        self.explain_loading = false;
    }

    /// Close the explain overlay
    fn close_explain(&mut self) {
        self.show_explain = false;
        self.explain_data = None;
        self.explain_loading = false;
        self.explain_error = None;
        self.explain_scroll = 0;
        self.pending_drill_down_query = None;
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        // Any mouse click closes help
        if self.show_help {
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.show_help = false;
            }
            return;
        }

        let x = mouse.column;
        let y = mouse.row;

        match mouse.kind {
            // Mouse wheel scrolling
            MouseEventKind::ScrollUp => {
                self.handle_scroll(-3);
            }
            MouseEventKind::ScrollDown => {
                self.handle_scroll(3);
            }

            // Left click
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is in tabs area
                if self.is_in_rect(x, y, self.layout.tabs_area) {
                    self.handle_tab_click(x);
                }
                // Check if click is in content area
                else if self.is_in_rect(x, y, self.layout.content_area) {
                    match self.active_tab {
                        Tab::Data => self.handle_data_click(y),
                        Tab::Chart => self.handle_chart_click(x, y),
                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }

    fn is_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }

    fn handle_scroll(&mut self, delta: i32) {
        match self.active_tab {
            Tab::Query => {
                if let Some(ref data) = self.data {
                    let max_scroll = get_query_line_count(data).saturating_sub(1);
                    if delta < 0 {
                        self.scroll_offset = self.scroll_offset.saturating_sub((-delta) as usize);
                    } else {
                        self.scroll_offset = (self.scroll_offset + delta as usize).min(max_scroll);
                    }
                }
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    let len = data.rows.len();
                    if len > 0 {
                        if delta < 0 {
                            self.selected_point =
                                self.selected_point.saturating_sub((-delta) as usize);
                        } else {
                            self.selected_point =
                                (self.selected_point + delta as usize).min(len - 1);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_tab_click(&mut self, x: u16) {
        // Calculate which tab was clicked based on x position
        // Tabs are roughly evenly spaced: "1:Query" "2:Mask" "3:Data" "4:Chart"
        let tabs_start = self.layout.tabs_area.x;
        let tabs_width = self.layout.tabs_area.width;
        let relative_x = x.saturating_sub(tabs_start);

        // Each tab takes roughly 1/4 of the width
        let tab_width = tabs_width / 4;
        let tab_index = (relative_x / tab_width.max(1)) as usize;

        if tab_index < 4 {
            self.active_tab = Tab::from_index(tab_index);
        }
    }

    fn handle_data_click(&mut self, y: u16) {
        if let Some(ref data) = self.data {
            // Calculate which row was clicked
            // Account for border (1) and header (2 lines including margin)
            let content_start = self.layout.content_area.y + 3;
            if y >= content_start {
                let row_index = (y - content_start) as usize;
                if row_index < data.rows.len() {
                    self.selected_point = row_index;
                }
            }
        }
    }

    fn handle_chart_click(&mut self, x: u16, y: u16) {
        if let Some(ref data) = self.data {
            if data.rows.is_empty() {
                return;
            }

            let chart_area = self.layout.chart_area;
            if !self.is_in_rect(x, y, chart_area) {
                return;
            }

            // For bar charts, calculate which bar was clicked
            // For line/scatter, find nearest point
            let chart_type = data.infer_chart_type();

            match chart_type {
                crate::data::ChartType::Bar => {
                    // Bar chart: divide chart width by number of bars
                    let chart_inner_x = chart_area.x + 1; // Account for border
                    let chart_inner_width = chart_area.width.saturating_sub(2);

                    if x >= chart_inner_x {
                        let relative_x = x - chart_inner_x;
                        // Each bar takes roughly equal width
                        let bar_width = chart_inner_width as usize / data.rows.len().max(1);
                        if bar_width > 0 {
                            let bar_index = relative_x as usize / bar_width;
                            if bar_index < data.rows.len() {
                                self.selected_point = bar_index;
                            }
                        }
                    }
                }
                crate::data::ChartType::Line | crate::data::ChartType::Scatter => {
                    // Line/scatter: find nearest point based on x position
                    let chart_inner_x = chart_area.x + 1;
                    let chart_inner_width = chart_area.width.saturating_sub(2);

                    if x >= chart_inner_x && chart_inner_width > 0 && data.rows.len() > 1 {
                        let relative_x = x - chart_inner_x;
                        let point_spacing =
                            chart_inner_width as f64 / (data.rows.len() - 1).max(1) as f64;
                        let point_index = (relative_x as f64 / point_spacing).round() as usize;
                        self.selected_point = point_index.min(data.rows.len() - 1);
                    }
                }
            }
        }
    }

    fn handle_up(&mut self) {
        match self.active_tab {
            Tab::Query => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    let len = data.rows.len();
                    if len > 0 {
                        self.selected_point = (self.selected_point + len - 1) % len;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_down(&mut self) {
        match self.active_tab {
            Tab::Query => {
                if let Some(ref data) = self.data {
                    let max_scroll = get_query_line_count(data).saturating_sub(1);
                    self.scroll_offset = (self.scroll_offset + 1).min(max_scroll);
                }
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    let len = data.rows.len();
                    if len > 0 {
                        self.selected_point = (self.selected_point + 1) % len;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_home(&mut self) {
        match self.active_tab {
            Tab::Query => {
                self.scroll_offset = 0;
            }
            Tab::Data | Tab::Chart => {
                self.selected_point = 0;
            }
            _ => {}
        }
    }

    fn handle_end(&mut self) {
        match self.active_tab {
            Tab::Query => {
                if let Some(ref data) = self.data {
                    self.scroll_offset = get_query_line_count(data).saturating_sub(1);
                }
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    if !data.rows.is_empty() {
                        self.selected_point = data.rows.len() - 1;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_page_up(&mut self) {
        match self.active_tab {
            Tab::Query => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    let len = data.rows.len();
                    if len > 0 {
                        self.selected_point = self.selected_point.saturating_sub(10);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_page_down(&mut self) {
        match self.active_tab {
            Tab::Query => {
                if let Some(ref data) = self.data {
                    let max_scroll = get_query_line_count(data).saturating_sub(1);
                    self.scroll_offset = (self.scroll_offset + 10).min(max_scroll);
                }
            }
            Tab::Data | Tab::Chart => {
                if let Some(ref data) = self.data {
                    let len = data.rows.len();
                    if len > 0 {
                        self.selected_point = (self.selected_point + 10).min(len - 1);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

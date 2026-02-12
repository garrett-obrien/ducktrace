use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use log::{debug, info};

use crate::data::{ChartData, ExplainData, HistoryEntry};
use crate::ui::query::get_query_line_count;
use crate::watcher::{get_data_path, load_data, load_history_entries};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home = 0,
    Query = 1,
    Mask = 2,
    Data = 3,
    Chart = 4,
}

impl Tab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Home,
            1 => Tab::Query,
            2 => Tab::Mask,
            3 => Tab::Data,
            4 => Tab::Chart,
            _ => Tab::Home,
        }
    }

    pub fn next(&self) -> Self {
        Tab::from_index((*self as usize + 1) % 5)
    }

    pub fn prev(&self) -> Self {
        Tab::from_index((*self as usize + 4) % 5)
    }
}

pub struct App {
    pub data: Option<ChartData>,
    pub active_tab: Tab,
    pub scroll_offset: usize,
    pub selected_point: usize,
    pub show_help: bool,
    pub running: bool,
    pub frame: u32,
    // Explain mode state
    pub show_explain: bool,
    pub explain_data: Option<ExplainData>,
    pub explain_loading: bool,
    pub explain_error: Option<String>,
    pub explain_scroll: usize,
    pub explain_selected_col: usize,
    pub explain_sort_column: Option<usize>,
    pub explain_sort_asc: bool,
    pub explain_sorted_indices: Vec<usize>,
    /// Pending drill-down query to execute (polled by main loop)
    pending_drill_down_query: Option<String>,
    // History state for Home tab data selector
    pub history: Vec<HistoryEntry>,
    pub history_selected: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            data: None,
            active_tab: Tab::Home,
            scroll_offset: 0,
            selected_point: 0,
            show_help: false,
            running: true,
            frame: 0,
            show_explain: false,
            explain_data: None,
            explain_loading: false,
            explain_error: None,
            explain_scroll: 0,
            explain_selected_col: 0,
            explain_sort_column: None,
            explain_sort_asc: true,
            explain_sorted_indices: Vec::new(),
            pending_drill_down_query: None,
            history: Vec::new(),
            history_selected: 0,
        }
    }

    pub fn on_data_update(&mut self, data: ChartData) {
        self.selected_point = 0;
        self.scroll_offset = 0;
        self.data = Some(data);
        self.active_tab = Tab::Query;
    }

    pub fn clear_data(&mut self) {
        let path = get_data_path();
        let _ = std::fs::remove_file(&path);
        self.data = None;
        self.selected_point = 0;
        self.scroll_offset = 0;
        self.active_tab = Tab::Home;
        self.close_explain();
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
                    let max_scroll = self.explain_sorted_indices.len().saturating_sub(1);
                    self.explain_scroll = (self.explain_scroll + 1).min(max_scroll);
                }
                KeyCode::PageUp => {
                    self.explain_scroll = self.explain_scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    let max_scroll = self.explain_sorted_indices.len().saturating_sub(1);
                    self.explain_scroll = (self.explain_scroll + 10).min(max_scroll);
                }
                KeyCode::Home => {
                    self.explain_scroll = 0;
                }
                KeyCode::End => {
                    let max_scroll = self.explain_sorted_indices.len().saturating_sub(1);
                    self.explain_scroll = max_scroll;
                }
                KeyCode::Left => {
                    if let Some(ref data) = self.explain_data {
                        let cols = data.columns.len();
                        if cols > 0 {
                            self.explain_selected_col = (self.explain_selected_col + cols - 1) % cols;
                        }
                    }
                }
                KeyCode::Right => {
                    if let Some(ref data) = self.explain_data {
                        let cols = data.columns.len();
                        if cols > 0 {
                            self.explain_selected_col = (self.explain_selected_col + 1) % cols;
                        }
                    }
                }
                KeyCode::Enter => {
                    self.toggle_explain_sort();
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c') => self.clear_data(),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Left => self.active_tab = self.active_tab.prev(),
            KeyCode::Right => self.active_tab = self.active_tab.next(),
            // Explain selected point / load history entry
            KeyCode::Char('x') => {
                if matches!(self.active_tab, Tab::Chart | Tab::Data) {
                    self.trigger_explain();
                }
            }
            KeyCode::Enter => {
                if matches!(self.active_tab, Tab::Chart | Tab::Data) {
                    self.trigger_explain();
                } else if self.active_tab == Tab::Home {
                    self.load_history_entry();
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if self.active_tab == Tab::Home {
                    self.delete_history_entry();
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
        let row_count = data.rows.len();
        self.explain_data = Some(data);
        self.explain_loading = false;
        self.explain_error = None;
        self.explain_selected_col = 0;
        self.explain_sort_column = None;
        self.explain_sort_asc = true;
        self.explain_sorted_indices = (0..row_count).collect();
    }

    fn toggle_explain_sort(&mut self) {
        let col = self.explain_selected_col;
        if let Some(current) = self.explain_sort_column {
            if current == col {
                if self.explain_sort_asc {
                    // Was ascending, flip to descending
                    self.explain_sort_asc = false;
                } else {
                    // Was descending, clear sort
                    self.explain_sort_column = None;
                    if let Some(ref data) = self.explain_data {
                        self.explain_sorted_indices = (0..data.rows.len()).collect();
                    }
                    self.explain_scroll = 0;
                    return;
                }
            } else {
                self.explain_sort_column = Some(col);
                self.explain_sort_asc = true;
            }
        } else {
            self.explain_sort_column = Some(col);
            self.explain_sort_asc = true;
        }
        self.apply_explain_sort();
        self.explain_scroll = 0;
    }

    fn apply_explain_sort(&mut self) {
        let Some(ref data) = self.explain_data else { return };
        let Some(col) = self.explain_sort_column else { return };
        let asc = self.explain_sort_asc;

        let mut indices: Vec<usize> = (0..data.rows.len()).collect();
        indices.sort_by(|&a, &b| {
            let va = data.rows[a].get(col);
            let vb = data.rows[b].get(col);
            let ord = cmp_json_values(va, vb);
            if asc { ord } else { ord.reverse() }
        });
        self.explain_sorted_indices = indices;
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
        self.explain_selected_col = 0;
        self.explain_sort_column = None;
        self.explain_sort_asc = true;
        self.explain_sorted_indices = Vec::new();
        self.pending_drill_down_query = None;
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.show_help {
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.show_help = false;
            }
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.handle_scroll(-3);
            }
            MouseEventKind::ScrollDown => {
                self.handle_scroll(3);
            }
            _ => {}
        }
    }

    fn handle_scroll(&mut self, delta: i32) {
        match self.active_tab {
            Tab::Home => {
                let len = self.history.len();
                if len > 0 {
                    if delta < 0 {
                        self.history_selected =
                            self.history_selected.saturating_sub((-delta) as usize);
                    } else {
                        self.history_selected =
                            (self.history_selected + delta as usize).min(len - 1);
                    }
                }
            }
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

    fn handle_up(&mut self) {
        match self.active_tab {
            Tab::Home => {
                let len = self.history.len();
                if len > 0 {
                    self.history_selected = (self.history_selected + len - 1) % len;
                }
            }
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
            Tab::Home => {
                let len = self.history.len();
                if len > 0 {
                    self.history_selected = (self.history_selected + 1) % len;
                }
            }
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
            Tab::Home => {
                self.history_selected = 0;
            }
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
            Tab::Home => {
                if !self.history.is_empty() {
                    self.history_selected = self.history.len() - 1;
                }
            }
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
            Tab::Home => {
                self.history_selected = self.history_selected.saturating_sub(10);
            }
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
            Tab::Home => {
                let len = self.history.len();
                if len > 0 {
                    self.history_selected = (self.history_selected + 10).min(len - 1);
                }
            }
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

    pub fn refresh_history(&mut self) {
        self.history = load_history_entries();
        if !self.history.is_empty() {
            self.history_selected = self.history_selected.min(self.history.len() - 1);
        } else {
            self.history_selected = 0;
        }
    }

    fn delete_history_entry(&mut self) {
        if self.history_selected >= self.history.len() {
            return;
        }
        let entry = &self.history[self.history_selected];
        let _ = std::fs::remove_file(&entry.path);
        self.refresh_history();
    }

    fn load_history_entry(&mut self) {
        if self.history_selected >= self.history.len() {
            return;
        }
        let entry = &self.history[self.history_selected];
        if let Ok(data) = load_data(&entry.path) {
            self.on_data_update(data);
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

/// Compare two optional JSON values for sorting.
/// Numbers sort numerically, strings lexicographically, nulls sort last.
fn cmp_json_values(
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(serde_json::Value::Null), Some(serde_json::Value::Null)) => Ordering::Equal,
        (Some(serde_json::Value::Null), _) => Ordering::Greater,
        (_, Some(serde_json::Value::Null)) => Ordering::Less,
        (Some(va), Some(vb)) => {
            // Try numeric comparison first
            if let (Some(na), Some(nb)) = (as_f64(va), as_f64(vb)) {
                return na.partial_cmp(&nb).unwrap_or(Ordering::Equal);
            }
            // Fall back to string comparison
            let sa = val_to_str(va);
            let sb = val_to_str(vb);
            sa.cmp(&sb)
        }
    }
}

fn as_f64(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn val_to_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}

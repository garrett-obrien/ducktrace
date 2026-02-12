use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Drill-down query template for explaining data points
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrillDown {
    /// Human-readable description of what the drill-down shows
    #[allow(dead_code)]
    pub description: String,
    /// Parameterized SQL query template (use {{x}}, {{y}} placeholders)
    #[serde(alias = "queryTemplate")]
    pub query_template: String,
    /// Maps placeholder names to field names (e.g., {"x": "category"})
    #[serde(default, alias = "paramMapping")]
    pub param_mapping: HashMap<String, String>,
}

/// Lineage information about data aggregation
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lineage {
    /// Aggregation function used (SUM, COUNT, AVG, etc.)
    pub aggregation: Option<String>,
    /// Original source column name before aggregation
    pub source_column: Option<String>,
    /// Source table name
    pub source_table: Option<String>,
    /// Columns used in GROUP BY clause
    pub group_by: Option<Vec<String>>,
}

/// Results from a drill-down query
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainData {
    /// Title for the explain panel
    pub title: String,
    /// ID of the command this responds to (for legacy Claude-based execution)
    #[allow(dead_code)]
    pub response_to_command: Option<String>,
    /// Column names from drill-down query
    pub columns: Vec<String>,
    /// Row data from drill-down query
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Total count of matching rows (before LIMIT)
    pub total_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartData {
    pub title: String,
    pub query: String,
    #[serde(alias = "x")]
    pub x_field: String,
    #[serde(alias = "y")]
    pub y_field: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    #[serde(alias = "chart_type")]
    pub chart_type: Option<String>,
    pub status: Option<String>,
    #[allow(dead_code)]
    pub error_message: Option<String>,
    pub truncated_from: Option<usize>,
    /// Drill-down configuration for explaining data points
    #[serde(alias = "drill_down")]
    pub drill_down: Option<DrillDown>,
    /// Data lineage information
    #[allow(dead_code)]
    pub lineage: Option<Lineage>,
    /// Explain data (populated when responding to drill-down command, legacy)
    #[allow(dead_code)]
    pub explain_data: Option<ExplainData>,
    /// Database name for drill-down queries (e.g., "orb_data_export")
    pub database: Option<String>,
    /// Timestamp in milliseconds
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub title: String,
    pub timestamp: u64,
    pub row_count: usize,
    #[allow(dead_code)]
    pub chart_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    Line,
    Bar,
    Scatter,
}

const MAX_ROWS: usize = 50;

impl ChartData {
    /// Truncate rows to MAX_ROWS, recording original count in `truncated_from`
    pub fn apply_row_limit(&mut self) {
        if self.rows.len() > MAX_ROWS {
            self.truncated_from = Some(self.rows.len());
            self.rows.truncate(MAX_ROWS);
            self.status = Some("truncated".to_string());
        }
    }

    /// Set timestamp to current time if not already present
    pub fn ensure_timestamp(&mut self) {
        if self.timestamp.is_none() {
            self.timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .ok();
        }
    }

    /// Infer the chart type based on data characteristics
    pub fn infer_chart_type(&self) -> ChartType {
        // Check explicit chart_type first
        if let Some(ref ct) = self.chart_type {
            match ct.to_lowercase().as_str() {
                "line" => return ChartType::Line,
                "bar" => return ChartType::Bar,
                "scatter" => return ChartType::Scatter,
                _ => {}
            }
        }

        // Infer from data
        if self.rows.is_empty() {
            return ChartType::Bar;
        }

        // Check if x values look like dates/times (line chart)
        let x_idx = self.get_x_index();
        if let Some(first_row) = self.rows.first() {
            if let Some(x_val) = first_row.get(x_idx) {
                if let Some(s) = x_val.as_str() {
                    // Check for date-like patterns
                    if s.contains('-') && s.len() >= 10 {
                        return ChartType::Line;
                    }
                }
            }
        }

        // Check if x values are numeric (could be scatter)
        let x_is_numeric = self.rows.iter().all(|row| {
            row.get(x_idx)
                .map(|v| v.is_number() || v.as_str().map(|s| s.parse::<f64>().is_ok()).unwrap_or(false))
                .unwrap_or(false)
        });

        if x_is_numeric {
            ChartType::Scatter
        } else {
            ChartType::Bar
        }
    }

    pub fn get_x_index(&self) -> usize {
        self.columns
            .iter()
            .position(|c| c == &self.x_field)
            .unwrap_or(0)
    }

    pub fn get_y_index(&self) -> usize {
        self.columns
            .iter()
            .position(|c| c == &self.y_field)
            .unwrap_or(1.min(self.columns.len().saturating_sub(1)))
    }

    pub fn get_x_value(&self, row: &[serde_json::Value]) -> String {
        let idx = self.get_x_index();
        row.get(idx)
            .map(value_to_string)
            .unwrap_or_default()
    }

    pub fn get_y_value(&self, row: &[serde_json::Value]) -> f64 {
        let idx = self.get_y_index();
        row.get(idx)
            .map(value_to_f64)
            .unwrap_or(0.0)
    }

    pub fn max_y(&self) -> f64 {
        self.rows
            .iter()
            .map(|row| self.get_y_value(row))
            .fold(0.0_f64, |a, b| a.max(b))
    }

    pub fn min_y(&self) -> f64 {
        self.rows
            .iter()
            .map(|row| self.get_y_value(row))
            .fold(f64::MAX, |a, b| a.min(b))
    }
}

pub fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => v.to_string(),
    }
}

pub fn value_to_f64(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_new_snake_case_format() {
        let json = r#"{
            "title": "Test",
            "query": "SELECT 1",
            "x": "month",
            "y": "revenue",
            "columns": ["month", "revenue"],
            "rows": [["2025-01", 100]],
            "chart_type": "line",
            "drill_down": {
                "description": "test",
                "query_template": "SELECT 1",
                "param_mapping": {"x": "month"}
            }
        }"#;
        let data: ChartData = serde_json::from_str(json).unwrap();
        assert_eq!(data.x_field, "month");
        assert_eq!(data.y_field, "revenue");
        assert_eq!(data.chart_type.as_deref(), Some("line"));
        assert!(data.drill_down.is_some());
        assert_eq!(data.drill_down.unwrap().query_template, "SELECT 1");
    }

    #[test]
    fn parse_old_camel_case_format() {
        let json = r#"{
            "title": "Test",
            "query": "SELECT 1",
            "xField": "month",
            "yField": "revenue",
            "columns": ["month", "revenue"],
            "rows": [["2025-01", 100]],
            "chartType": "bar",
            "drillDown": {
                "description": "test",
                "queryTemplate": "SELECT 1",
                "paramMapping": {"x": "month"}
            }
        }"#;
        let data: ChartData = serde_json::from_str(json).unwrap();
        assert_eq!(data.x_field, "month");
        assert_eq!(data.y_field, "revenue");
        assert_eq!(data.chart_type.as_deref(), Some("bar"));
        assert!(data.drill_down.is_some());
    }

    #[test]
    fn apply_row_limit_truncates() {
        let json = r#"{
            "title": "Test",
            "query": "SELECT 1",
            "x": "id",
            "y": "val",
            "columns": ["id", "val"],
            "rows": []
        }"#;
        let mut data: ChartData = serde_json::from_str(json).unwrap();
        // Add 60 rows
        for i in 0..60 {
            data.rows.push(vec![
                serde_json::Value::Number(i.into()),
                serde_json::Value::Number(i.into()),
            ]);
        }
        assert_eq!(data.rows.len(), 60);
        data.apply_row_limit();
        assert_eq!(data.rows.len(), 50);
        assert_eq!(data.truncated_from, Some(60));
        assert_eq!(data.status.as_deref(), Some("truncated"));
    }

    #[test]
    fn ensure_timestamp_sets_when_missing() {
        let json = r#"{
            "title": "Test",
            "query": "SELECT 1",
            "x": "id",
            "y": "val",
            "columns": ["id", "val"],
            "rows": []
        }"#;
        let mut data: ChartData = serde_json::from_str(json).unwrap();
        assert!(data.timestamp.is_none());
        data.ensure_timestamp();
        assert!(data.timestamp.is_some());
        assert!(data.timestamp.unwrap() > 1_000_000_000_000); // millis
    }

    #[test]
    fn ensure_timestamp_preserves_existing() {
        let json = r#"{
            "title": "Test",
            "query": "SELECT 1",
            "x": "id",
            "y": "val",
            "columns": ["id", "val"],
            "rows": [],
            "timestamp": 1234567890000
        }"#;
        let mut data: ChartData = serde_json::from_str(json).unwrap();
        data.ensure_timestamp();
        assert_eq!(data.timestamp, Some(1234567890000));
    }
}

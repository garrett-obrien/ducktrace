use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Drill-down query template for explaining data points
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartData {
    pub title: String,
    pub query: String,
    #[serde(alias = "xField")]
    pub x_field: String,
    #[serde(alias = "yField")]
    pub y_field: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub chart_type: Option<String>,
    pub status: Option<String>,
    #[allow(dead_code)]
    pub error_message: Option<String>,
    pub truncated_from: Option<usize>,
    /// Drill-down configuration for explaining data points
    pub drill_down: Option<DrillDown>,
    /// Data lineage information
    #[allow(dead_code)]
    pub lineage: Option<Lineage>,
    /// Explain data (populated when responding to drill-down command, legacy)
    #[allow(dead_code)]
    pub explain_data: Option<ExplainData>,
    /// Database name for drill-down queries (e.g., "orb_data_export")
    pub database: Option<String>,
    /// Timestamp in milliseconds (Date.now() from JS)
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

impl ChartData {
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

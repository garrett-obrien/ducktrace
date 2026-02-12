use anyhow::{Context, Result};
use duckdb::{types::ValueRef, Connection, Row};
use log::{debug, info};

/// Query executor that connects to MotherDuck via embedded DuckDB
pub struct QueryExecutor {
    _marker: (),
}

impl QueryExecutor {
    /// Verify MotherDuck connection is possible
    pub fn connect() -> Result<Self> {
        debug!("Opening MotherDuck connection for verification");
        let _conn = Connection::open("md:")
            .context("Failed to connect to MotherDuck. Ensure MOTHERDUCK_TOKEN is set.")?;
        debug!("MotherDuck connection verified");
        Ok(Self { _marker: () })
    }

    /// Execute a drill-down query and return results as (columns, rows)
    pub fn execute_drill_down(
        &self,
        query: &str,
    ) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
        debug!("Opening fresh MotherDuck connection for query");
        let conn = Connection::open("md:")
            .context("Failed to connect to MotherDuck")?;
        debug!("Connection opened");

        debug!("Preparing query");
        let mut stmt = conn.prepare(query).context("Failed to prepare query")?;
        debug!("Calling query()");
        let mut rows_result = stmt.query([]).context("Failed to execute query")?;
        debug!("Query started");

        // Collect all rows first, then get column info
        let mut all_rows: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut col_count = 0;

        while let Some(row) = rows_result.next()? {
            if all_rows.is_empty() {
                debug!("Got first row");
            }
            // Determine column count from first row
            if col_count == 0 {
                // Try to find column count by probing
                col_count = Self::probe_column_count(row);
                debug!("Detected {} columns", col_count);
            }

            let row_values = Self::extract_row(row, col_count)?;
            all_rows.push(row_values);

            if all_rows.len().is_multiple_of(20) {
                debug!("Fetched {} rows so far", all_rows.len());
            }
        }

        // Now get column names (should work after iterating)
        debug!("Getting column names");
        let mut columns = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let name = stmt
                .column_name(i)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("col_{}", i));
            columns.push(name);
        }
        info!("Query complete: {} columns, {} rows", columns.len(), all_rows.len());

        Ok((columns, all_rows))
    }

    /// Probe to find column count by trying to access columns
    fn probe_column_count(row: &Row) -> usize {
        for i in 0..100 {
            if row.get_ref(i).is_err() {
                return i;
            }
        }
        100
    }

    /// Extract all values from a row
    fn extract_row(row: &Row, col_count: usize) -> Result<Vec<serde_json::Value>> {
        let mut values = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val = Self::extract_value(row, i)?;
            values.push(val);
        }
        Ok(values)
    }

    /// Extract a single value from a row
    fn extract_value(row: &Row, idx: usize) -> Result<serde_json::Value> {
        let val = match row.get_ref(idx)? {
            ValueRef::Null => serde_json::Value::Null,
            ValueRef::Boolean(b) => serde_json::json!(b),
            ValueRef::TinyInt(n) => serde_json::json!(n),
            ValueRef::SmallInt(n) => serde_json::json!(n),
            ValueRef::Int(n) => serde_json::json!(n),
            ValueRef::BigInt(n) => serde_json::json!(n),
            ValueRef::HugeInt(n) => serde_json::json!(n.to_string()),
            ValueRef::UTinyInt(n) => serde_json::json!(n),
            ValueRef::USmallInt(n) => serde_json::json!(n),
            ValueRef::UInt(n) => serde_json::json!(n),
            ValueRef::UBigInt(n) => serde_json::json!(n),
            ValueRef::Float(f) => serde_json::json!(f),
            ValueRef::Double(f) => serde_json::json!(f),
            ValueRef::Decimal(d) => serde_json::json!(d.to_string()),
            ValueRef::Timestamp(_, n) => serde_json::json!(format_timestamp_micros(n)),
            ValueRef::Text(s) => serde_json::json!(String::from_utf8_lossy(s).to_string()),
            ValueRef::Blob(b) => serde_json::json!(format!("<blob {} bytes>", b.len())),
            ValueRef::Date32(days) => serde_json::json!(format_date_days(days)),
            ValueRef::Time64(_, micros) => serde_json::json!(format_time_micros(micros)),
            ValueRef::Interval { months, days, nanos } => {
                serde_json::json!(format!("{}m {}d {}ns", months, days, nanos))
            }
            ValueRef::List(list, _) => serde_json::json!(format!("{:?}", list)),
            ValueRef::Enum(e, _) => serde_json::json!(format!("{:?}", e)),
            ValueRef::Struct(s, _) => serde_json::json!(format!("{:?}", s)),
            ValueRef::Array(a, _) => serde_json::json!(format!("{:?}", a)),
            ValueRef::Map(m, _) => serde_json::json!(format!("{:?}", m)),
            ValueRef::Union(u, _) => serde_json::json!(format!("{:?}", u)),
        };
        Ok(val)
    }
}

fn format_timestamp_micros(micros: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    if micros >= 0 {
        let duration = Duration::from_micros(micros as u64);
        if let Some(datetime) = UNIX_EPOCH.checked_add(duration) {
            if let Ok(elapsed) = datetime.duration_since(UNIX_EPOCH) {
                let secs = elapsed.as_secs();
                let days = secs / 86400;
                let day_secs = secs % 86400;
                let hours = day_secs / 3600;
                let mins = (day_secs % 3600) / 60;
                let secs = day_secs % 60;
                let (year, month, day) = days_to_ymd(days as i64 + 719468);
                return format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                    year, month, day, hours, mins, secs
                );
            }
        }
    }
    micros.to_string()
}

fn format_date_days(days: i32) -> String {
    let (year, month, day) = days_to_ymd(days as i64 + 719468);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn format_time_micros(micros: i64) -> String {
    let total_secs = micros / 1_000_000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

fn days_to_ymd(z: i64) -> (i32, u32, u32) {
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

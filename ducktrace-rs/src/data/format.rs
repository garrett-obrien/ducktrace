/// Format a numeric value for display
pub fn format_number(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.2}", value)
    }
}

/// Format a value as currency
pub fn format_currency(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("${:.1}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("${:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("${:.1}K", value / 1_000.0)
    } else {
        format!("${:.2}", value)
    }
}

/// Format a value as percentage
pub fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

/// Detect and format a value based on field name hints
pub fn format_value(value: f64, field_name: &str) -> String {
    let lower = field_name.to_lowercase();

    if lower.contains("percent") || lower.contains("pct") || lower.contains("rate") {
        format_percent(value)
    } else if lower.contains("price") || lower.contains("cost") || lower.contains("revenue")
        || lower.contains("amount") || lower.contains("$") {
        format_currency(value)
    } else {
        format_number(value)
    }
}

/// Truncate a string to fit within max_width, adding ellipsis if needed
pub fn truncate_string(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width <= 3 {
        s.chars().take(max_width).collect()
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1_500_000_000.0), "1.5B");
        assert_eq!(format_number(2_500_000.0), "2.5M");
        assert_eq!(format_number(1_500.0), "1.5K");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(3.14159), "3.14");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(1_000_000.0), "$1.0M");
        assert_eq!(format_currency(2_500.0), "$2.5K");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }
}

"""Value formatting utilities for currency, percentages, and numbers."""


def format_display_value(value, field: str = "") -> str:
    """Format a value for display based on field name hints."""
    if value is None:
        return "-"

    field_lower = field.lower()

    # Currency detection
    if any(
        kw in field_lower
        for kw in ["revenue", "price", "cost", "amount", "$", "sales"]
    ):
        try:
            num = float(value)
            return f"${num:,.0f}"
        except (ValueError, TypeError):
            return str(value)

    # Percentage detection
    if any(kw in field_lower for kw in ["rate", "percent", "%"]):
        try:
            num = float(value)
            return f"{num:.1f}%"
        except (ValueError, TypeError):
            return str(value)

    # Number formatting
    if isinstance(value, (int, float)):
        return f"{value:,}"

    return str(value)


def format_axis_value(value: float) -> str:
    """Format large numbers for Y-axis labels."""
    abs_value = abs(value)
    if abs_value >= 1_000_000_000:
        return f"${value / 1_000_000_000:.1f}B"
    elif abs_value >= 1_000_000:
        return f"${value / 1_000_000:.1f}M"
    elif abs_value >= 1_000:
        return f"${value / 1_000:.1f}K"
    return f"${value:.0f}"

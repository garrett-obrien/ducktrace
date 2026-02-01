"""View components for the TUI."""

import math
import re

import plotext as plt
import sqlparse
from rich.console import Console, Group
from rich.panel import Panel
from rich.syntax import Syntax
from rich.table import Table
from rich.text import Text

from .formatting import format_axis_value, format_display_value


def format_sql(query: str) -> str:
    """Format SQL query with proper indentation and line breaks."""
    if not query:
        return ""
    return sqlparse.format(
        query,
        reindent=True,
        keyword_case="upper",
        indent_width=2,
    )


def strip_ansi(text: str) -> str:
    """Strip ANSI escape codes from text."""
    return re.sub(r'\x1b\[[0-9;]*m', '', text)


def calculate_nice_ticks(y_max: float, num_ticks: int = 5) -> list[float]:
    """Calculate evenly-spaced 'nice' tick values starting from 0."""
    if y_max <= 0:
        return [0]

    raw_step = y_max / (num_ticks - 1)
    magnitude = 10 ** math.floor(math.log10(raw_step))

    normalized = raw_step / magnitude
    if normalized <= 1:
        nice_step = magnitude
    elif normalized <= 2:
        nice_step = 2 * magnitude
    elif normalized <= 5:
        nice_step = 5 * magnitude
    else:
        nice_step = 10 * magnitude

    ticks = []
    current = 0.0
    while current <= y_max * 1.01:  # Small buffer to include max
        ticks.append(current)
        current += nice_step

    return ticks


# Duck ASCII art for waiting state animation
DUCK_FEET_WAITING = [
    r"""  \\    //
   \\  //
 ___\\//___
|  _/  \_  |
| / \  / \ |
|/   \/   \|""",
    r"""   \\  //
    \\//
  __\\//___
 |  _/ \_  |
 | / \ / \ |
 |/   \/  \|""",
    r"""  \\    //
   \\  //
 ___\\//___
|  _/  \_  |
| / \  / \ |
|/   \/   \|""",
    r"""    \\//
   __\\__
  /  /\  \
 |  /  \  |
 | / \/ \ |
 |/      \|""",
]


def get_duck_frame(frame_index: int) -> str:
    """Get duck animation frame."""
    return DUCK_FEET_WAITING[frame_index % len(DUCK_FEET_WAITING)]


def render_waiting_state(frame: int) -> Panel:
    """Render the waiting state with duck animation."""
    duck_art = get_duck_frame(frame)

    content = Text()
    content.append(duck_art, style="yellow")
    content.append("\n\n")
    content.append("Waiting for query...", style="bold white")
    content.append("\n\n")
    content.append("Run /explain-chart in Claude Code\n", style="dim")
    content.append("or use the explain-chart skill with a MotherDuck query", style="dim")

    return Panel(
        content,
        title="[yellow]<(o)>[/yellow] MotherDuck Explain Chart",
        border_style="yellow",
        padding=(1, 2),
    )


def render_query_view(query: str, content_height: int = None) -> Panel:
    """Render the Query view with SQL syntax highlighting."""
    formatted_query = format_sql(query)
    syntax = Syntax(
        formatted_query,
        "sql",
        theme="monokai",
        word_wrap=True,
        padding=(1, 2),
    )

    content = Group(
        Text("Step 1: Query", style="white"),
        Text(),
        syntax,
        Text(),
        Text("This query aggregates data for visualization.", style="dim"),
    )

    return Panel(content, border_style="bright_black", padding=(1, 2), height=content_height)


def render_mask_view(x_field: str, y_field: str, columns: list, content_height: int = None) -> Panel:
    """Render the Mask view showing column -> axis mapping."""
    table = Table(show_header=True, header_style="bold white", box=None)
    table.add_column("Column", width=20)
    table.add_column("Role", width=15)

    for col in columns:
        is_x = col == x_field
        is_y = col == y_field

        if is_x:
            role = "-> X Axis"
            role_style = "cyan bold"
            col_style = "white"
        elif is_y:
            role = "-> Y Axis"
            role_style = "magenta bold"
            col_style = "white"
        else:
            role = "(excluded)"
            role_style = "dim"
            col_style = "dim strike"

        table.add_row(Text(col, style=col_style), Text(role, style=role_style))

    content = Group(
        Text("Step 2: Column Mapping", style="white"),
        Text(),
        table,
        Text(),
        Text(f"{x_field} drives the X axis, {y_field} drives the Y axis.", style="dim"),
    )

    return Panel(content, border_style="bright_black", padding=(1, 2), height=content_height)


def render_data_view(
    rows: list, columns: list, x_field: str, y_field: str, scroll_offset: int, visible_rows: int, selected_index: int = None, content_height: int = None
) -> Panel:
    """Render the Data view with scrollable table and row highlighting."""
    display_columns = [c for c in [x_field, y_field] if c in columns]

    table = Table(show_header=True, header_style="bold cyan", box=None)
    for i, col in enumerate(display_columns):
        table.add_column(col, width=30, no_wrap=True, overflow="ellipsis")

    visible_data = rows[scroll_offset : scroll_offset + visible_rows]

    for row_idx, row in enumerate(visible_data):
        actual_idx = scroll_offset + row_idx
        is_selected = selected_index is not None and actual_idx == selected_index

        row_values = []
        for i, col in enumerate(display_columns):
            val = row.get(col, "") if isinstance(row, dict) else row[columns.index(col)] if col in columns else ""
            if i == 0:
                cell_text = str(val)
            else:
                cell_text = format_display_value(val, col)

            if is_selected:
                row_values.append(Text(cell_text, style="bold yellow"))
            else:
                row_values.append(cell_text)

        # Add marker for selected row
        if is_selected:
            row_values[0] = Text(f"◆ {row_values[0].plain if isinstance(row_values[0], Text) else row_values[0]}", style="bold yellow")

        table.add_row(*row_values)

    # Selection info
    scroll_info = Text()
    if selected_index is not None and 0 <= selected_index < len(rows):
        scroll_info.append("\n")
        scroll_info.append(f"◆ Row {selected_index + 1} of {len(rows)}", style="yellow")
    elif len(rows) > visible_rows:
        scroll_info.append("\n")
        if scroll_offset > 0:
            scroll_info.append("^ ", style="dim")
        scroll_info.append(
            f"Showing {scroll_offset + 1}-{min(scroll_offset + visible_rows, len(rows))} of {len(rows)}",
            style="dim",
        )
        if scroll_offset + visible_rows < len(rows):
            scroll_info.append(" v", style="dim")

    content = Group(
        Text(f"Step 3: Data ({len(rows)} rows)", style="white"),
        Text(),
        table,
        scroll_info,
    )

    return Panel(content, border_style="bright_black", padding=(1, 2), height=content_height)


def truncate_label(label: str, max_len: int = 8) -> str:
    """Truncate label with ellipsis if too long."""
    if len(label) <= max_len:
        return label
    return label[:max_len-1] + "…"


def infer_chart_type(rows: list, x_field: str, y_field: str, explicit_type: str = None) -> str:
    """Infer the best chart type based on data characteristics."""
    if explicit_type:
        return explicit_type

    if not rows:
        return "line"

    # Get sample values
    sample = rows[0] if isinstance(rows[0], dict) else dict(zip([x_field, y_field], rows[0]))
    x_val = sample.get(x_field)
    y_val = sample.get(y_field)

    # Check if X looks like dates/time series
    x_str = str(x_val) if x_val else ""
    if any(
        pattern in x_str
        for pattern in ["-", "/", "2020", "2021", "2022", "2023", "2024", "2025", "2026"]
    ):
        return "line"

    # Check if X is numeric (scatter plot candidate)
    try:
        float(x_val)
        float(y_val)
        # Both numeric - could be scatter
        return "scatter"
    except (ValueError, TypeError):
        pass

    # Categorical X with numeric Y -> bar chart
    try:
        float(y_val)
        return "bar"
    except (ValueError, TypeError):
        pass

    return "line"


def render_chart_view(
    rows: list,
    title: str,
    x_field: str,
    y_field: str,
    chart_type: str = None,
    width: int = 60,
    height: int = 15,
    selected_index: int = None,
) -> Panel:
    """Render the Chart view using plotext."""
    if not rows:
        return Panel(
            Text("No data available for chart", style="dim"),
            border_style="bright_black",
            padding=(1, 2),
        )

    # Extract data
    x_labels = []
    y_values = []

    for row in rows:
        if isinstance(row, dict):
            x_labels.append(str(row.get(x_field, "")))
            try:
                y_values.append(float(row.get(y_field, 0)))
            except (ValueError, TypeError):
                y_values.append(0)
        else:
            x_labels.append(str(row[0]) if row else "")
            try:
                y_values.append(float(row[1]) if len(row) > 1 else 0)
            except (ValueError, TypeError):
                y_values.append(0)

    # Infer chart type if not specified
    actual_chart_type = infer_chart_type(rows, x_field, y_field, chart_type)

    # Reset plotext completely
    plt.clf()  # Clear figure
    plt.cld()  # Clear data

    # Use clear theme (no colors) - Rich will handle styling
    plt.theme("clear")

    # Disable right Y-axis and upper X-axis to prevent artifacts
    plt.yaxes(left=True, right=False)
    plt.xaxes(lower=True, upper=False)

    # Set conservative size - account for Rich panel borders
    plot_width = min(width, 70)
    plot_height = min(height, 20)
    plt.plotsize(plot_width, plot_height)

    # Plot data using indices
    x_indices = list(range(len(y_values)))

    if actual_chart_type == "bar":
        plt.bar(x_indices, y_values)
    elif actual_chart_type == "scatter":
        # For scatter, try to use numeric x values
        try:
            x_numeric = [float(x) for x in x_labels]
            plt.scatter(x_numeric, y_values)
        except (ValueError, TypeError):
            plt.scatter(x_indices, y_values)
    else:  # line
        plt.plot(x_indices, y_values)

    # Highlight selected point with a marker (using ◆ for consistent size)
    if selected_index is not None and 0 <= selected_index < len(y_values):
        if actual_chart_type == "scatter":
            try:
                x_numeric = [float(x) for x in x_labels]
                plt.scatter([x_numeric[selected_index]], [y_values[selected_index]], marker="◆", color="yellow")
            except (ValueError, TypeError):
                plt.scatter([x_indices[selected_index]], [y_values[selected_index]], marker="◆", color="yellow")
        else:
            plt.scatter([x_indices[selected_index]], [y_values[selected_index]], marker="◆", color="yellow")

    # Title and x-axis label (y-axis label added vertically after build)
    plt.title(title or "Chart")
    plt.xlabel(x_field)

    # X-axis: show subset of labels to avoid overlap
    if len(x_labels) <= 8:
        plt.xticks(x_indices, [truncate_label(lbl, 8) for lbl in x_labels])
    else:
        step = len(x_labels) // 6
        ticks = list(range(0, len(x_labels), step))
        if ticks[-1] != len(x_labels) - 1:
            ticks.append(len(x_labels) - 1)
        plt.xticks(ticks, [truncate_label(x_labels[i], 8) for i in ticks])

    # Y-axis: use nice evenly-spaced ticks starting from 0
    if y_values:
        y_max = max(y_values)
        y_ticks = calculate_nice_ticks(y_max, num_ticks=5)
        y_tick_labels = [format_axis_value(v) for v in y_ticks]
        plt.ylim(0, y_ticks[-1])  # Set explicit range from 0 to nice max
        plt.yticks(y_ticks, y_tick_labels)

    # Build chart string and strip ANSI codes (Rich handles styling)
    chart_str = strip_ansi(plt.build())

    # Add y-axis label vertically on the left side (rotated 90 degrees)
    chart_lines = chart_str.split('\n')
    if len(chart_lines) > 2 and y_field:
        # Center the label vertically, pad with spaces
        label_height = len(chart_lines)
        label_chars = list(y_field[:label_height])  # Truncate if too long
        padding_top = (label_height - len(label_chars)) // 2

        new_lines = []
        for i, line in enumerate(chart_lines):
            label_idx = i - padding_top
            if 0 <= label_idx < len(label_chars):
                new_lines.append(f"{label_chars[label_idx]} {line}")
            else:
                new_lines.append(f"  {line}")
        chart_str = '\n'.join(new_lines)

    # Build styled chart text with yellow marker for selected point
    chart_text = Text()
    for char in chart_str:
        if char == "◆":
            chart_text.append(char, style="bold yellow")
        else:
            chart_text.append(char, style="cyan")

    # Build detail text for selected point or show general info
    if selected_index is not None and 0 <= selected_index < len(rows):
        x_val = x_labels[selected_index]
        y_val = y_values[selected_index]
        y_formatted = format_display_value(y_val, y_field)
        detail_text = Text(f"◆ Selected: {x_field}={x_val}, {y_field}={y_formatted}", style="yellow")
    else:
        detail_text = Text(f"X: {x_field} | Y: {y_field} | Points: {len(rows)} | Type: {actual_chart_type}", style="dim")

    # Wrap in Rich content
    content = Group(
        Text("Step 4: Chart", style="white"),
        Text(),
        chart_text,
        Text(),
        detail_text,
    )

    return Panel(content, border_style="bright_black", padding=(1, 2))

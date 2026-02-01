# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Claude Code skill that generates interactive charts with data lineage ("Explain") features. It integrates with MotherDuck MCP to query data, then produces charts where users can right-click any data point to trace it back through the SQL query.

## Commands

```bash
uv sync                      # Install Python dependencies
uv run ducktrace     # Start terminal UI (for split-terminal viewing)
node src/generate.js         # Run standalone HTML generator
```

## Architecture

### Two Output Modes

1. **Browser (HTML)** - Standalone HTML file with interactive Chart.js visualization and explain panel
2. **Terminal (TUI)** - Python Rich/plotext-based terminal UI that watches for data updates in real-time

### Key Entry Points

- `src/explain-chart-mcp.js` - Primary entry point for Claude Code skill invocation. Takes JSON config with MotherDuck MCP response and generates both HTML output and TUI data file
- `src/generate.js` - Alternative generator supporting multiple input formats (simple, columnar, legacy MCP)
- `src/tui.py` - TUI entry point using Rich Live display

### Data Flow

```
MotherDuck MCP query → explain-chart-mcp.js → HTML file + ~/.claude/explain-chart/current.json
                                                             ↓
                                              TUI watches and auto-refreshes
```

### Python TUI Structure

```
src/
├── tui.py              # Main app with Rich Live display, tab navigation, keyboard input
├── views.py            # Query, Mask, Data, Chart view renderers using Rich + plotext
├── watcher.py          # watchdog file watcher for live updates
└── formatting.py       # Currency/percent/number formatters
```

### Chart Types

The TUI supports automatic chart type inference:
- **line** - Time series data (dates on X axis)
- **bar** - Categorical X with numeric Y
- **scatter** - Two numeric columns

Override via config: `"chart_type": "bar"`

### Config Format (explain-chart-mcp.js)

```json
{
  "title": "Chart Title",
  "x": "x_column_name",
  "y": "y_column_name",
  "query": "SELECT ...",
  "columns": ["col1", "col2"],
  "rows": [["val1", 100], ["val2", 200]],
  "output": "/path/to/output.html",
  "chart_type": "line"
}
```

### TUI Data File

Written to `~/.claude/explain-chart/current.json` - the TUI watches this file and auto-refreshes when it changes.


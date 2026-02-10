# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

DuckTrace is a Claude Code skill that generates interactive charts with data lineage ("Explain") features. It integrates with MotherDuck MCP to query data, then produces explorable visualizations where users can select any data point and drill down into the underlying rows.

## Commands

```bash
# Build the Rust TUI
cd ducktrace-rs && cargo build --release

# Run the TUI (watches ~/.claude/ducktrace/current.json)
./ducktrace-rs/target/release/ducktrace

# Lint
cd ducktrace-rs && cargo clippy

# Run tests
cd ducktrace-rs && cargo test

# Skill entry point (JSON arg, used by Claude Code)
node src/ducktrace-mcp.js '<JSON config>'
```

## Environment Setup

Copy `.env.example` to `.env` and set your MotherDuck token. The TUI reads this for drill-down queries.

```bash
cp .env.example .env
# Edit .env and set MOTHERDUCK_TOKEN
```

## Architecture

### Output Mode

**Terminal (TUI)** — `ducktrace-rs/` — Rust/ratatui with mouse support, auto-refresh, and drill-down queries via DuckDB.

### Key Entry Points

- `src/ducktrace-mcp.js` — Primary entry point for Claude Code skill invocation. Takes a single JSON argument with chart config + MCP response data. Writes TUI data to `~/.claude/ducktrace/current.json`.

### Data Flow

```
MotherDuck MCP query → ducktrace-mcp.js → ~/.claude/ducktrace/current.json
                                                             ↓
                                              TUI watches and auto-refreshes
                                                             ↓
                                              User presses 'x' → drill-down query via DuckDB
```

### Project Structure

```
ducktrace/
├── CLAUDE.md
├── SKILL.md                # Claude Code skill definition (triggers, workflow, examples)
├── README.md
├── package.json            # Node.js config (type: module)
├── .env.example            # MOTHERDUCK_TOKEN template
├── .gitignore
├── src/
│   └── ducktrace-mcp.js    # Skill entry point (JSON arg → TUI data)
└── ducktrace-rs/
    ├── Cargo.toml          # ratatui, crossterm, duckdb, tokio, notify, serde
    └── src/
        ├── main.rs         # Entry point, async runtime, event loop
        ├── app.rs          # App state, keyboard + mouse handling
        ├── db.rs           # MotherDuck connection via DuckDB for drill-down queries
        ├── watcher.rs      # File watcher (notify crate)
        ├── data/
        │   ├── mod.rs
        │   ├── model.rs    # ChartData struct, chart type inference
        │   └── format.rs   # Number/currency formatting
        └── ui/
            ├── mod.rs      # Main render function, layout
            ├── tabs.rs     # Tab bar rendering
            ├── query.rs    # SQL query view with syntax highlighting
            ├── mask.rs     # Column mapping table
            ├── data.rs     # Data table with row selection
            ├── chart.rs    # Chart rendering (line/bar/scatter)
            ├── explain.rs  # Drill-down results overlay
            └── help.rs     # Help overlay
```

### Chart Types

Auto-inferred from data, or set explicitly via `"chart_type"`:
- **line** — Time series (dates on X axis)
- **bar** — Categorical X with numeric Y
- **scatter** — Two numeric columns

### Config Format (ducktrace-mcp.js)

```json
{
  "title": "Chart Title",
  "x": "x_column_name",
  "y": "y_column_name",
  "query": "SELECT ...",
  "database": "db_name",
  "columns": ["col1", "col2"],
  "rows": [["val1", 100], ["val2", 200]],
  "chart_type": "line",
  "drillDown": {
    "description": "Show detail rows",
    "query_template": "SELECT * FROM {{database}}.table WHERE x = '{{x}}' LIMIT 100",
    "param_mapping": {"x": "x_column_name"}
  }
}
```

Required fields: `title`, `x`, `y`, `query`, `columns`, `rows`. Row limit: 50 (auto-truncated).

### TUI Data File

Written to `~/.claude/ducktrace/current.json` — the TUI watches this file and auto-refreshes when it changes.

## Keyboard Shortcuts (TUI)

| Key | Action |
|-----|--------|
| `←` `→` | Switch between tabs |
| `1`-`4` | Jump to tab (Query/Mask/Data/Chart) |
| `↑` `↓` | Scroll/select within tab |
| `Home` `End` | Jump to first/last |
| `PgUp` `PgDn` | Page scroll |
| `x` | Execute drill-down on selected data point |
| `Esc` | Close drill-down overlay |
| `c` | Clear data file |
| `?` | Toggle help overlay |
| `q` | Quit |

## Mouse Support

| Action | Effect |
|--------|--------|
| Click on tab | Switch to that tab |
| Click on data row | Select that row |
| Click on chart bar/point | Select that data point |
| Scroll wheel (Query tab) | Scroll SQL query |
| Scroll wheel (Data/Chart tab) | Change selected row/point |

## SQL Syntax Highlighting

The Query tab features syntax highlighting:
- **Magenta (bold)** — Keywords (`SELECT`, `FROM`, `WHERE`, etc.)
- **Blue** — Functions (`SUM`, `COUNT`, `DATE_TRUNC`, etc.)
- **Green** — String literals
- **Yellow** — Numbers
- **Gray (italic)** — Comments (`--`)
- **Red** — Operators (`=`, `<>`, `+`, etc.)
- **Cyan** — Identifiers (column/table names)

## Key Dependencies

**Rust TUI:** ratatui 0.29, crossterm 0.28, duckdb 1.4 (bundled), tokio, notify 7, serde
**Node.js:** ES modules (`"type": "module"` in package.json), no runtime dependencies

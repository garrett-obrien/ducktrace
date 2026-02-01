# DuckTrace

Interactive charts with data lineage — right-click any data point to trace it back through your SQL query.

A Claude Code skill that integrates with [MotherDuck](https://motherduck.com/) MCP to query data and generate explorable visualizations.

## Requirements

- [Claude Code](https://claude.ai/code) CLI
- [MotherDuck MCP](https://github.com/motherduckdb/mcp-server-motherduck) configured in Claude Code
- Python 3.11+ with [uv](https://github.com/astral-sh/uv)
- Node.js 18+

## Installation

```bash
git clone https://github.com/goblinfactory/ducktrace.git
cd ducktrace
uv sync
```

## Usage

### As a Claude Code Skill

Invoke the skill in Claude Code after running a MotherDuck query:

```
/explain-chart
```

This generates an interactive HTML chart and updates the TUI data file.

### Terminal UI

Run the TUI in a split terminal to see charts update in real-time:

```bash
uv run ducktrace
```

The TUI watches `~/.claude/explain-chart/current.json` and auto-refreshes when new data arrives.

### Keyboard Controls (TUI)

| Key | Action |
|-----|--------|
| `1-4` | Switch views (Query, Mask, Data, Chart) |
| `←` `→` | Navigate data points |
| `↑` `↓` | Scroll data table |
| `q` | Quit |

## How It Works

```
MotherDuck MCP query
        ↓
  /explain-chart skill
        ↓
  ┌─────────────────────────────────┐
  │  HTML file (browser)            │
  │  + ~/.claude/explain-chart/     │
  │    current.json (TUI)           │
  └─────────────────────────────────┘
        ↓
  TUI watches & auto-refreshes
```

### Views

1. **Query** — SQL with syntax highlighting
2. **Mask** — Column-to-axis mapping
3. **Data** — Scrollable data table with row selection
4. **Chart** — plotext visualization with point selection

### Chart Types

Automatically inferred from data:
- **line** — Time series (dates on X axis)
- **bar** — Categorical X with numeric Y
- **scatter** — Two numeric columns

Override with config: `"chart_type": "bar"`

## Development

```bash
uv sync                      # Install Python dependencies
uv run ducktrace     # Start terminal UI
node src/generate.js         # Run standalone HTML generator
```

### Project Structure

```
src/
├── explain-chart-mcp.js    # Claude Code skill entry point
├── generate.js             # HTML generator
├── tui.py                  # TUI main app (Rich Live display)
├── views.py                # View renderers (Query, Mask, Data, Chart)
├── watcher.py              # File watcher for live updates
└── formatting.py           # Value formatters
```

## License

MIT

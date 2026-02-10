🦆🦆under duckvelopment🦆🦆

# DuckTrace

Interactive charts with data lineage — select any data point to drill down into the underlying rows.

A Claude Code skill that integrates with [MotherDuck](https://motherduck.com/) MCP to query data and generate explorable visualizations.

## Requirements

- [Claude Code](https://claude.ai/code) CLI
- [MotherDuck MCP](https://github.com/motherduckdb/mcp-server-motherduck) configured in Claude Code
- Rust (for building the TUI)
- Node.js 18+

## Installation

```bash
git clone https://github.com/goblinfactory/ducktrace.git
cd ducktrace/ducktrace-rs
cargo build --release
```

## Usage

### As a Claude Code Skill

Invoke the skill in Claude Code after running a MotherDuck query:

```
/ducktrace
```

This generates an interactive chart and updates the TUI data file.

### Terminal UI

Run the TUI in a split terminal to see charts update in real-time:

```bash
./ducktrace-rs/target/release/ducktrace
```

The TUI watches `~/.claude/ducktrace/current.json` and auto-refreshes when new data arrives.

### Keyboard Controls (TUI)

| Key | Action |
|-----|--------|
| `1-4` | Switch views (Query, Mask, Data, Chart) |
| `←` `→` | Navigate between tabs |
| `↑` `↓` | Scroll/select within tab |
| `x` | Drill-down on selected data point |
| `Esc` | Close drill-down overlay |
| `?` | Toggle help |
| `q` | Quit |

## How It Works

```
MotherDuck MCP query
        ↓
  /ducktrace skill
        ↓
  ┌─────────────────────────────────┐
  │  ~/.claude/ducktrace/           │
  │    current.json (TUI data)      │
  └─────────────────────────────────┘
        ↓
  TUI watches & auto-refreshes
        ↓
  Press 'x' to drill-down
        ↓
  TUI queries MotherDuck directly
```

### Views

1. **Query** — SQL with syntax highlighting
2. **Mask** — Column-to-axis mapping
3. **Data** — Scrollable data table with row selection
4. **Chart** — Bar/line/scatter visualization with point selection

### Chart Types

Automatically inferred from data:
- **line** — Time series (dates on X axis)
- **bar** — Categorical X with numeric Y
- **scatter** — Two numeric columns

Override with config: `"chart_type": "bar"`

## Development

```bash
cd ducktrace-rs
cargo build --release    # Build TUI
cargo run --release      # Run TUI
```

### Project Structure

```
ducktrace-rs/
├── src/
│   ├── main.rs         # Entry point, event loop
│   ├── app.rs          # App state, input handling
│   ├── db.rs           # MotherDuck drill-down queries
│   ├── watcher.rs      # File watcher
│   ├── data/           # Data models
│   └── ui/             # UI components

src/
└── ducktrace-mcp.js        # Claude Code skill entry point
```

## License

MIT

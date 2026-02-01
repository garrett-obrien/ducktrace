---
name: explain-chart
description: Generate interactive charts with "Explain" feature - right-click any data point to trace it back to the SQL query. Requires MotherDuck MCP.
---

# Explain Chart Skill

Generate a line chart from a MotherDuck query with built-in data lineage. Users can explore:

1. **Query** - The SQL that produced the data
2. **Mask** - Which columns map to X/Y axes
3. **Data** - The result set
4. **Value** - The specific row highlighted with connector lines

## When to Use

Trigger when user asks for: "explain chart", "chart with explain", "visualize with lineage", or any chart request where they want to trace data back to SQL.

## Viewing Options

**IMPORTANT: In Claude Code, always use the TUI output by default.** Only generate HTML if the user explicitly requests it (e.g., "save as HTML", "generate HTML file", "open in browser").

### Option 1: Terminal TUI (Default in Claude Code)
The default output for Claude Code users. The TUI runs in a split terminal alongside the main session and automatically updates when new chart data is generated.

### Option 2: Browser (HTML)
A standalone HTML file with an interactive explain panel. Only use this when the user explicitly requests HTML output or browser viewing. Right-click any data point to see the 4-step explanation.

**Setup (one-time):**
```bash
# In the explain-chart skill directory
uv sync

# Open a split terminal pane and run:
uv run explain-chart-tui
# Alternative: uv run python -m src.tui
```

**Split Terminal Layout:**
```
+-------------------------------+---------------------------------------+
|  Claude Code                  |  MotherDuck Explain Chart             |
|                               |                                       |
|  claude> show revenue by      |  <(o)> MotherDuck Explain Chart       |
|          month                |                                       |
|                               |  [Query] - Mask - Data - Chart        |
|  Running query...             |  +-----------------------------------+ |
|                               |  | SELECT strftime('%Y-%m',         | |
|  [Results appear]             |  |        order_date) AS month,     | |
|                               |  |        SUM(amount) AS revenue    | |
|                               |  | FROM orders                      | |
|                               |  | GROUP BY 1 ORDER BY 1            | |
|                               |  +-----------------------------------+ |
|                               |                                       |
|                               |  <- -> tabs  <arrows> scroll  q quit  |
+-------------------------------+---------------------------------------+
```

The TUI automatically updates when you run the skill in Claude Code.

## Workflow

### Step 1: Get Parameters from User

If not provided, ask for:
- Database name
- What to visualize (Claude writes the SQL)
- Chart title (or infer from context)
- X and Y fields (or infer from query)

### Step 2: Call MotherDuck MCP

```
MotherDuck:query(database="<db>", sql="<query>")
```

Response contains `columns` (array of names) and `rows` (array of arrays).

### Step 3: Generate Chart

Pass the MCP response directly to the generator as a single JSON config:

```bash
node /mnt/skills/user/explain-chart/src/explain-chart-mcp.js '<JSON>'
```

The JSON config combines chart params with MCP response:

```json
{
  "title": "Chart Title",
  "x": "x_field_name",
  "y": "y_field_name",
  "query": "SELECT ... (the SQL you ran)",
  "columns": ["col1", "col2"],
  "rows": [["val1", 100], ["val2", 200]],
  "output": "/home/claude/chart.html"
}
```

### Step 4: Output Results

**In Claude Code (default):** The generator automatically writes data to `~/.claude/explain-chart/current.json`. If the user has the TUI running in a split terminal (`npm run tui`), it will automatically refresh to show the new chart. Simply confirm the chart was generated and remind the user to check their TUI pane.

**HTML output (only when explicitly requested):** Use `present_files` to share the generated HTML file with the user.

## Complete Example (TUI - Default)

User: "show me revenue by month for 2025 with explain chart"

1. **Claude calls MCP:**
   ```
   MotherDuck:query(
     database="sales_db",
     sql="SELECT strftime(date_trunc('month', order_date), '%Y-%m') AS month, SUM(amount) AS revenue FROM orders WHERE order_date >= '2025-01-01' GROUP BY 1 ORDER BY 1"
   )
   ```

2. **MCP returns:**
   ```json
   {
     "columns": ["month", "revenue"],
     "rows": [["2025-01", 19000097.60], ["2025-02", 18457859.77], ...]
   }
   ```

3. **Claude runs generator (no output path = TUI only):**
   ```bash
   node /mnt/skills/user/explain-chart/src/explain-chart-mcp.js '{
     "title": "2025 Revenue by Month",
     "x": "month",
     "y": "revenue",
     "query": "SELECT strftime(date_trunc('month', order_date), '%Y-%m') AS month, SUM(amount) AS revenue FROM orders WHERE order_date >= '2025-01-01' GROUP BY 1 ORDER BY 1",
     "columns": ["month", "revenue"],
     "rows": [["2025-01", 19000097.60], ["2025-02", 18457859.77]]
   }'
   ```

4. **Claude confirms:** "Chart generated! Check your TUI pane to explore the data."

## Example (HTML - Only When Requested)

User: "show me revenue by month and save it as an HTML file"

Same steps 1-2, then:

3. **Claude runs generator with output path:**
   ```bash
   node /mnt/skills/user/explain-chart/src/explain-chart-mcp.js '{
     ...
     "output": "/home/claude/revenue_chart.html"
   }'
   ```

4. **Claude presents file** to user

## Config Reference

| Field | Required | Description |
|-------|----------|-------------|
| `title` | Yes | Chart title |
| `x` | Yes | Column name for X axis |
| `y` | Yes | Column name for Y axis |
| `query` | Yes | The SQL query (displayed in explain panel) |
| `columns` | Yes | Column names from MCP response |
| `rows` | Yes | Row data from MCP response |
| `output` | No | HTML output path. Only include when user explicitly requests HTML. Omit for TUI-only output. |
| `chart_type` | No | Chart type: "line", "bar", or "scatter". Auto-inferred if omitted (dates->line, categorical->bar, numeric x numeric->scatter). |

## Output

### TUI Data (Default in Claude Code)
Data is written to `~/.claude/explain-chart/current.json`. The TUI watches this file and auto-refreshes, showing:
- Query tab with syntax-highlighted SQL
- Mask tab showing column → axis mapping
- Data tab with scrollable result table
- Chart tab with ASCII visualization

### HTML File (Only When Requested)
When user explicitly requests HTML output, generates a standalone file with:
- Interactive line chart
- Right-click context menu on data points
- 4-step explain panel showing query -> mask -> data -> value
- Connector lines linking table rows to chart points
- Zero external dependencies

## TUI Keyboard Controls

| Key | Action |
|-----|--------|
| `<-` / `->` | Navigate between tabs |
| `1-4` | Jump to specific tab |
| `Up/Down` | Scroll data table |
| `q` | Quit TUI |

## Chart Types

The TUI supports three chart types that can be explicitly set or auto-inferred:

| Type | When Inferred | Best For |
|------|---------------|----------|
| `line` | X contains dates/timestamps | Time series data |
| `bar` | Categorical X with numeric Y | Comparisons across categories |
| `scatter` | Both X and Y are numeric | Correlation analysis |

## Requirements

- MotherDuck MCP must be connected
- Node.js available in environment (for HTML generation)
- Python 3.14+ with uv for TUI (`uv sync` in skill directory)

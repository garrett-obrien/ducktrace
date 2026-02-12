---
name: ducktrace
description: Interactive TUI charts with drill-down - select any data point to query the underlying rows. Requires MotherDuck MCP.
---

# DuckTrace

Generate interactive charts from MotherDuck queries with built-in drill-down capability. Users can explore:

1. **Query** - The SQL that produced the data
2. **Mask** - Which columns map to X/Y axes
3. **Data** - The result set
4. **Value** - The specific row highlighted with connector lines

## When to Use

Trigger when user asks for: "ducktrace", "trace chart", "chart with drill-down", "visualize with lineage", or any chart request where they want to drill into underlying data.

## Viewing

The TUI runs in a split terminal alongside the main Claude Code session and automatically updates when new chart data is generated.

**Setup (one-time):**
```bash
# Build the Rust TUI
cd ducktrace-rs && cargo build --release

# Open a split terminal pane and run:
./ducktrace-rs/target/release/ducktrace
```

**Split Terminal Layout:**
```
+-------------------------------+---------------------------------------+
|  Claude Code                  |  DuckTrace                            |
|                               |                                       |
|  claude> show revenue by      |  🦆 DuckTrace: Revenue by Month       |
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
|                               |  ←→: tabs | ↑↓: select | x: drill-down|
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

**Time series queries should use `ORDER BY ... DESC`** so the most recent data appears first in the Data tab. The TUI automatically selects the most recent point on load.

Response contains `columns` (array of names) and `rows` (array of arrays).

### Step 3: Write Chart Data

Write the JSON config directly to the TUI data file. The TUI handles row truncation (50 max) and timestamping automatically.

```bash
mkdir -p ~/.claude/ducktrace && cat > ~/.claude/ducktrace/current.json << 'EOF'
{
  "title": "Chart Title",
  "x": "x_column_name",
  "y": "y_column_name",
  "query": "SELECT ... (the SQL you ran)",
  "database": "db_name",
  "columns": ["col1", "col2"],
  "rows": [["val1", 100], ["val2", 200]],
  "chart_type": "line",
  "drill_down": {
    "description": "Show detail rows",
    "query_template": "SELECT * FROM {{database}}.table WHERE col = '{{x}}' LIMIT 100",
    "param_mapping": {"x": "x_column_name"}
  }
}
EOF
```

Keep rows under 50 for optimal display — the TUI will truncate if needed.

### Step 4: Output Results

The data file at `~/.claude/ducktrace/current.json` is watched by the TUI, which auto-refreshes when it changes. The TUI also archives each chart to history automatically. Confirm the chart was generated and remind the user to check their TUI pane.

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

3. **Claude writes chart data:**
   ```bash
   mkdir -p ~/.claude/ducktrace && cat > ~/.claude/ducktrace/current.json << 'EOF'
   {
     "title": "2025 Revenue by Month",
     "x": "month",
     "y": "revenue",
     "query": "SELECT strftime(date_trunc('month', order_date), '%Y-%m') AS month, SUM(amount) AS revenue FROM orders WHERE order_date >= '2025-01-01' GROUP BY 1 ORDER BY 1",
     "columns": ["month", "revenue"],
     "rows": [["2025-01", 19000097.60], ["2025-02", 18457859.77]]
   }
   EOF
   ```

4. **Claude confirms:** "Chart generated! Check your TUI pane to explore the data."

## Config Reference

| Field | Required | Description |
|-------|----------|-------------|
| `title` | Yes | Chart title |
| `x` | Yes | Column name for X axis |
| `y` | Yes | Column name for Y axis |
| `query` | Yes | The SQL query (displayed in explain panel) |
| `database` | Yes | Database name for drill-down queries |
| `columns` | Yes | Column names from MCP response |
| `rows` | Yes | Row data from MCP response |
| `chart_type` | No | Chart type: `"line"`, `"bar"`, or `"scatter"`. Auto-inferred if omitted (dates->line, categorical->bar, numeric x numeric->scatter). |
| `drill_down` | No | Drill-down template for explaining data points. See Drill-Down Templates section. |

## Output

Data is written to `~/.claude/ducktrace/current.json`. The TUI watches this file and auto-refreshes, showing:
- Query tab with syntax-highlighted SQL
- Mask tab showing column → axis mapping
- Data tab with scrollable result table
- Chart tab with ASCII visualization

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

## Drill-Down Templates

When generating a chart from an aggregation query, **always** include a `drillDown` object that enables users to explore the underlying detail rows.

### How Drill-Down Works

1. User selects a data point in the TUI (e.g., "2025-01" with revenue $19M)
2. TUI substitutes placeholders in the template with actual values
3. TUI executes the query against MotherDuck
4. Results display in an overlay showing the detail rows

### Generating Drill-Down Templates

Analyze the original query to understand:
- **Aggregations**: SUM, COUNT, AVG, MIN, MAX, etc.
- **GROUP BY clause**: Which columns define the aggregation buckets
- **Source table**: Where the detail rows live
- **Filter conditions**: Any WHERE clauses that should carry over

Then generate a template that retrieves the underlying rows for a selected data point.

### Template Format

```json
{
  "drill_down": {
    "description": "Human-readable description of what this shows",
    "query_template": "SELECT * FROM {{database}}.table WHERE condition = '{{x}}' LIMIT 100",
    "param_mapping": {
      "x": "x_column_name",
      "y": "y_column_name"
    }
  }
}
```

### Placeholder Reference

| Placeholder | Description | Example Value |
|-------------|-------------|---------------|
| `{{database}}` | Database name from config | `sales_db` |
| `{{x}}` | Selected point's X value | `2025-01` |
| `{{y}}` | Selected point's Y value | `19000097.60` |

### Examples

**Example 1: Monthly Revenue Aggregation**

Original query:
```sql
SELECT strftime('%Y-%m', order_date) AS month, SUM(amount) AS revenue
FROM orders
GROUP BY 1
ORDER BY 1
```

Drill-down template:
```json
{
  "drill_down": {
    "description": "Show orders for selected month",
    "query_template": "SELECT order_id, order_date, customer_name, amount FROM {{database}}.orders WHERE strftime('%Y-%m', order_date) = '{{x}}' ORDER BY amount DESC LIMIT 100",
    "param_mapping": {"x": "month"}
  }
}
```

**Example 2: Category Sales Count**

Original query:
```sql
SELECT category, COUNT(*) AS order_count
FROM products
JOIN order_items USING (product_id)
GROUP BY 1
```

Drill-down template:
```json
{
  "drill_down": {
    "description": "Show orders for selected category",
    "query_template": "SELECT p.product_name, oi.quantity, oi.unit_price FROM {{database}}.products p JOIN {{database}}.order_items oi USING (product_id) WHERE p.category = '{{x}}' LIMIT 100",
    "param_mapping": {"x": "category"}
  }
}
```

**Example 3: Customer Spending (AVG aggregation)**

Original query:
```sql
SELECT customer_segment, AVG(total_spent) AS avg_spending
FROM customers
GROUP BY 1
```

Drill-down template:
```json
{
  "drill_down": {
    "description": "Show customers in selected segment",
    "query_template": "SELECT customer_name, email, total_spent FROM {{database}}.customers WHERE customer_segment = '{{x}}' ORDER BY total_spent DESC LIMIT 100",
    "param_mapping": {"x": "customer_segment"}
  }
}
```

### Best Practices

1. **Always include LIMIT**: Use `LIMIT 100` to prevent overwhelming the TUI with too many rows
2. **Order meaningfully**: Order by the aggregated column DESC to show most significant rows first
3. **Select useful columns**: Include identifying columns (IDs, names) plus the aggregated value
4. **Preserve filters**: If the original query has WHERE clauses, include them in the drill-down
5. **Match the grouping**: The drill-down WHERE clause should filter on the same column(s) as GROUP BY
6. **Use {{database}} prefix**: Always prefix table names with `{{database}}.` for cross-database safety

### Complete Example with Drill-Down

User: "show me revenue by month for 2025 with explain chart"

1. **Claude calls MCP:**
   ```
   MotherDuck:query(
     database="sales_db",
     sql="SELECT strftime('%Y-%m', order_date) AS month, SUM(amount) AS revenue FROM orders WHERE order_date >= '2025-01-01' GROUP BY 1 ORDER BY 1"
   )
   ```

2. **Claude writes chart data with drill-down:**
   ```bash
   mkdir -p ~/.claude/ducktrace && cat > ~/.claude/ducktrace/current.json << 'EOF'
   {
     "title": "2025 Revenue by Month",
     "x": "month",
     "y": "revenue",
     "database": "sales_db",
     "query": "SELECT strftime('%Y-%m', order_date) AS month, SUM(amount) AS revenue FROM orders WHERE order_date >= '2025-01-01' GROUP BY 1 ORDER BY 1",
     "columns": ["month", "revenue"],
     "rows": [["2025-01", 19000097.60], ["2025-02", 18457859.77]],
     "drill_down": {
       "description": "Show orders for selected month",
       "query_template": "SELECT order_id, order_date, customer_name, amount FROM {{database}}.orders WHERE strftime('%Y-%m', order_date) = '{{x}}' AND order_date >= '2025-01-01' ORDER BY amount DESC LIMIT 100",
       "param_mapping": {"x": "month"}
     }
   }
   EOF
   ```

3. **User presses 'x' on January data point** → TUI executes:
   ```sql
   SELECT order_id, order_date, customer_name, amount
   FROM sales_db.orders
   WHERE strftime('%Y-%m', order_date) = '2025-01'
     AND order_date >= '2025-01-01'
   ORDER BY amount DESC
   LIMIT 100
   ```

4. **TUI displays overlay** with the 100 largest orders from January

## Requirements

- MotherDuck MCP must be connected
- Rust TUI binary (`cargo build --release` in ducktrace-rs/)

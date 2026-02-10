#!/usr/bin/env node

/**
 * ducktrace-mcp.js
 *
 * Claude Code skill entry point for DuckTrace.
 * Takes chart config and MCP response data as a single JSON argument.
 *
 * Writes TUI data to ~/.claude/ducktrace/current.json
 *
 * Usage:
 *   node ducktrace-mcp.js '<JSON config>'
 *
 * Config format:
 *   {
 *     "title": "Chart Title",
 *     "x": "x_field",
 *     "y": "y_field",
 *     "query": "SELECT ...",
 *     "database": "db_name",
 *     "columns": ["col1", "col2"],
 *     "rows": [["val1", 100], ...],
 *     "chart_type": "line",    // optional: "line", "bar", or "scatter" (auto-inferred if omitted)
 *     "drillDown": {           // optional: drill-down template for data exploration
 *       "description": "Show detail rows",
 *       "query_template": "SELECT * FROM {{database}}.table WHERE x = '{{x}}' LIMIT 100",
 *       "param_mapping": {"x": "x_field"}
 *     }
 *   }
 *
 * Example:
 *   node ducktrace-mcp.js '{"title":"Revenue","x":"month","y":"revenue","query":"SELECT...","columns":["month","revenue"],"rows":[["2025-01",100]]}'
 */

import fs from 'fs';
import path from 'path';
import os from 'os';

// Data limits
const MAX_ROWS = 50;
const MIN_ROWS = 2;

// Get config from argument
const configArg = process.argv[2];
if (!configArg) {
  console.error('Usage: node ducktrace-mcp.js \'<JSON config>\'');
  console.error('Config: { title, x, y, query, columns, rows }');
  process.exit(1);
}

let config;
try {
  config = JSON.parse(configArg);
} catch (e) {
  console.error('Failed to parse config JSON:', e.message);
  process.exit(1);
}

// Validate required fields
const required = ['title', 'x', 'y', 'query', 'columns', 'rows'];
for (const r of required) {
  if (!config[r]) {
    console.error(`Missing required field: ${r}`);
    process.exit(1);
  }
}

// Apply row limits and track truncation
let rows = config.rows;
let status = 'success';
let truncatedFrom = null;

if (rows.length > MAX_ROWS) {
  console.warn(`Truncating ${rows.length} rows to ${MAX_ROWS} for optimal display`);
  truncatedFrom = rows.length;
  rows = rows.slice(0, MAX_ROWS);
  status = 'truncated';
}

if (rows.length < MIN_ROWS) {
  console.warn(`Warning: Only ${rows.length} row(s) - charts work best with ${MIN_ROWS}+ rows`);
}

// Write TUI data file for Claude Code split terminal viewing
const tuiDataDir = path.join(os.homedir(), '.claude', 'ducktrace');
const tuiDataFile = path.join(tuiDataDir, 'current.json');

try {
  // Ensure directory exists
  if (!fs.existsSync(tuiDataDir)) {
    fs.mkdirSync(tuiDataDir, { recursive: true });
  }

  // Write TUI-compatible JSON
  const tuiData = {
    title: config.title,
    query: config.query,
    xField: config.x,
    yField: config.y,
    columns: config.columns,
    rows: rows,  // Use truncated rows
    chartType: config.chart_type || null,
    status: status,
    truncatedFrom: truncatedFrom,
    timestamp: Date.now(),
    // Database name for drill-down queries (e.g., "orb_data_export")
    database: config.database || null,
    // Drill-down configuration (optional, provided by Claude)
    drillDown: config.drillDown || config.drill_down || null,
    // Data lineage information (optional, provided by Claude)
    lineage: config.lineage || null,
    // Explain data for drill-down responses (optional)
    explainData: config.explainData || config.explain_data || null,
  };

  fs.writeFileSync(tuiDataFile, JSON.stringify(tuiData, null, 2));
  console.log(`TUI data: ${tuiDataFile}`);
} catch (e) {
  // TUI data write is optional, don't fail if it doesn't work
  console.log(`Note: Could not write TUI data: ${e.message}`);
}

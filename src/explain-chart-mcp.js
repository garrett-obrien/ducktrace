#!/usr/bin/env node

/**
 * explain-chart-mcp.js
 *
 * Streamlined wrapper for Claude to generate explain-charts.
 * Takes chart config and MCP response data as a single JSON argument.
 *
 * Outputs:
 *   1. HTML file for browser viewing (with interactive explain panel)
 *   2. JSON file for TUI viewing in Claude Code split terminal
 *
 * Usage:
 *   node explain-chart-mcp.js '<JSON config>'
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
 *     "output": "chart.html",  // optional, defaults to /home/claude/chart.html
 *     "chart_type": "line"     // optional: "line", "bar", or "scatter" (auto-inferred if omitted)
 *   }
 *
 * Example:
 *   node explain-chart-mcp.js '{"title":"Revenue","x":"month","y":"revenue","query":"SELECT...","columns":["month","revenue"],"rows":[["2025-01",100]]}'
 */

import fs from 'fs';
import path from 'path';
import os from 'os';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Get config from argument
const configArg = process.argv[2];
if (!configArg) {
  console.error('Usage: node explain-chart-mcp.js \'<JSON config>\'');
  console.error('Config: { title, x, y, query, columns, rows, output? }');
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

// Find template
const skillDir = '/mnt/skills/user/explain-chart/src';
const templatePath = fs.existsSync(path.join(skillDir, 'template.html'))
  ? path.join(skillDir, 'template.html')
  : path.join(__dirname, 'template.html');

if (!fs.existsSync(templatePath)) {
  console.error('Cannot find template.html');
  process.exit(1);
}

const template = fs.readFileSync(templatePath, 'utf8');

// Convert columnar format to array of objects
function columnarToObjects(columns, rows) {
  return rows.map(row => {
    const obj = {};
    columns.forEach((col, i) => {
      // Convert string numbers to actual numbers for numeric columns
      let val = row[i];
      if (typeof val === 'string' && !isNaN(val) && val.trim() !== '') {
        val = parseFloat(val);
      }
      obj[col] = val;
    });
    return obj;
  });
}

const data = columnarToObjects(config.columns, config.rows);

// Generate HTML
let html = template
  .replace(/\{\{TITLE\}\}/g, config.title)
  .replace('{{DATA}}', JSON.stringify(data))
  .replace('{{QUERY}}', JSON.stringify(config.query))
  .replace('{{X_FIELD}}', JSON.stringify(config.x))
  .replace('{{Y_FIELD}}', JSON.stringify(config.y));

// Write HTML output only if output path is provided
if (config.output) {
  fs.writeFileSync(config.output, html);
  console.log(`Generated: ${config.output}`);
}

// Write TUI data file for Claude Code split terminal viewing
const tuiDataDir = path.join(os.homedir(), '.claude', 'explain-chart');
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
    rows: config.rows,
    chartType: config.chart_type || null,
    timestamp: Date.now()
  };

  fs.writeFileSync(tuiDataFile, JSON.stringify(tuiData, null, 2));
  console.log(`TUI data: ${tuiDataFile}`);
} catch (e) {
  // TUI data write is optional, don't fail if it doesn't work
  console.log(`Note: Could not write TUI data: ${e.message}`);
}

#!/usr/bin/env node

/**
 * Generate HTML from an explain-chart spec
 *
 * Simple format (from skill wrapper):
 * {
 *   "query": "SELECT ...",
 *   "database": "my_db",
 *   "result": [...],
 *   "chart_config": { type, title, x, y }
 * }
 *
 * Legacy MCP format (backward compatible):
 * {
 *   "mcp_request": { ... },
 *   "mcp_response": { ... },
 *   "chart_config": { ... }
 * }
 *
 * Usage: node generate.js <input.json> [output.html]
 *    or: echo '<json>' | node generate.js - [output.html]
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Read template
const templatePath = path.join(__dirname, 'template.html');
const template = fs.readFileSync(templatePath, 'utf8');

// Get input - either from file or stdin
const inputArg = process.argv[2];
if (!inputArg) {
  console.error('Usage: node generate.js <input.json> [output.html]');
  console.error('   or: echo \'<json>\' | node generate.js - [output.html]');
  process.exit(1);
}

let specJson;
if (inputArg === '-') {
  // Read from stdin
  specJson = fs.readFileSync(0, 'utf8');
} else {
  specJson = fs.readFileSync(inputArg, 'utf8');
}

const spec = JSON.parse(specJson);

// Convert columnar format to array of objects
function columnarToObjects(columns, rows) {
  return rows.map(row => {
    const obj = {};
    columns.forEach((col, i) => {
      obj[col] = row[i];
    });
    return obj;
  });
}

// Extract components based on format
let query, data, chartConfig;

if (spec.query && spec.result && spec.chart_config) {
  // Simple format (from skill wrapper) - result is array of objects
  query = spec.query;
  data = spec.result;
  chartConfig = spec.chart_config;

} else if (spec.query && spec.columns && spec.rows && spec.chart_config) {
  // Columnar format (direct from MCP response)
  query = spec.query;
  data = columnarToObjects(spec.columns, spec.rows);
  chartConfig = spec.chart_config;

} else if (spec.mcp_request && spec.mcp_response && spec.chart_config) {
  // Legacy MCP format
  query = spec.mcp_request.params?.arguments?.sql || '';

  const responseContent = spec.mcp_response.result?.content?.[0]?.text;
  if (responseContent) {
    try {
      data = JSON.parse(responseContent);
    } catch (e) {
      console.error('Failed to parse MCP response data:', e.message);
      process.exit(1);
    }
  } else {
    console.error('No data found in MCP response');
    process.exit(1);
  }

  chartConfig = spec.chart_config;

} else if (spec.query && spec.result && spec.x && spec.y) {
  // Minimal format
  query = spec.query;
  data = spec.result;
  chartConfig = {
    type: 'line',
    title: spec.title || 'Chart',
    x: spec.x,
    y: spec.y
  };

} else {
  console.error('Invalid input format. Expected:');
  console.error('  { query, result, chart_config: { type, title, x, y } }');
  process.exit(1);
}

// Validate
if (!query) {
  console.error('Missing SQL query');
  process.exit(1);
}
if (!data || !Array.isArray(data) || data.length === 0) {
  console.error('Missing or invalid data array');
  process.exit(1);
}
if (!chartConfig.x || !chartConfig.y) {
  console.error('Missing x or y field in chart config');
  process.exit(1);
}

// Generate HTML
let html = template
  .replace(/\{\{TITLE\}\}/g, chartConfig.title || 'Chart')
  .replace('{{DATA}}', JSON.stringify(data))
  .replace('{{QUERY}}', JSON.stringify(query))
  .replace('{{X_FIELD}}', JSON.stringify(chartConfig.x))
  .replace('{{Y_FIELD}}', JSON.stringify(chartConfig.y));

// Write output
let outputFile;
if (process.argv[3]) {
  outputFile = process.argv[3];
} else if (inputArg === '-') {
  outputFile = 'output.html';
} else {
  outputFile = inputArg.replace('.json', '.html');
}

fs.writeFileSync(outputFile, html);
console.log(`Generated: ${outputFile}`);

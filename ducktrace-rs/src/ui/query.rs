use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::data::ChartData;

/// SQL token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenType {
    Keyword,
    Function,
    String,
    Number,
    Comment,
    Operator,
    Identifier,
    Whitespace,
    Punctuation,
}

/// A token with its type and text
struct Token<'a> {
    token_type: TokenType,
    text: &'a str,
}

/// SQL keywords to highlight
const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS",
    "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON",
    "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "UNION", "ALL",
    "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "DROP",
    "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA", "ALTER", "ADD", "COLUMN",
    "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "CONSTRAINT", "UNIQUE",
    "DEFAULT", "CHECK", "CASCADE", "RESTRICT", "NULLS", "FIRST", "LAST",
    "ASC", "DESC", "DISTINCT", "TOP", "CASE", "WHEN", "THEN", "ELSE", "END",
    "EXISTS", "BETWEEN", "LIKE", "ILIKE", "ESCAPE", "CAST", "CONVERT",
    "TRUE", "FALSE", "WITH", "RECURSIVE", "OVER", "PARTITION", "WINDOW",
    "ROWS", "RANGE", "UNBOUNDED", "PRECEDING", "FOLLOWING", "CURRENT", "ROW",
    "FILTER", "WITHIN", "ROLLUP", "CUBE", "GROUPING", "SETS",
    "INTERSECT", "EXCEPT", "MINUS", "FETCH", "NEXT", "ONLY", "PERCENT",
    "FOR", "LATERAL", "NATURAL", "USING", "QUALIFY",
];

/// SQL aggregate/window functions
const SQL_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MIN", "MAX", "TOTAL",
    "ROW_NUMBER", "RANK", "DENSE_RANK", "NTILE", "LAG", "LEAD",
    "FIRST_VALUE", "LAST_VALUE", "NTH_VALUE",
    "COALESCE", "NULLIF", "IFNULL", "NVL", "IIF",
    "UPPER", "LOWER", "TRIM", "LTRIM", "RTRIM", "SUBSTR", "SUBSTRING",
    "LENGTH", "LEN", "CONCAT", "REPLACE", "REVERSE", "SPLIT_PART",
    "TO_CHAR", "TO_DATE", "TO_TIMESTAMP", "TO_NUMBER",
    "DATE", "TIME", "TIMESTAMP", "DATETIME", "INTERVAL",
    "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND",
    "DATE_TRUNC", "DATE_PART", "EXTRACT", "DATEDIFF", "DATEADD",
    "NOW", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP",
    "ABS", "ROUND", "FLOOR", "CEIL", "CEILING", "MOD", "POWER", "SQRT",
    "LOG", "LN", "EXP", "SIGN", "RANDOM",
    "ARRAY", "ARRAY_AGG", "STRING_AGG", "LISTAGG", "GROUP_CONCAT",
    "JSON", "JSON_EXTRACT", "JSON_VALUE", "JSON_QUERY",
    "STRFTIME", "STRPTIME", "EPOCH", "AGE",
    "LIST", "STRUCT", "MAP", "UNNEST", "GENERATE_SERIES",
    "ARG_MAX", "ARG_MIN", "ANY_VALUE", "BIT_AND", "BIT_OR", "BIT_XOR",
    "BOOL_AND", "BOOL_OR", "CORR", "COVAR_POP", "COVAR_SAMP",
    "STDDEV", "STDDEV_POP", "STDDEV_SAMP", "VARIANCE", "VAR_POP", "VAR_SAMP",
    "PERCENTILE_CONT", "PERCENTILE_DISC", "MODE", "MEDIAN",
    "TRY_CAST", "TYPEOF", "COLUMNS", "EXCLUDE", "REPLACE",
];

/// SQL operators
const SQL_OPERATORS: &[&str] = &[
    "=", "<>", "!=", "<", ">", "<=", ">=", "+", "-", "*", "/", "%",
    "||", "->", "->>", "::", "@", "#", "&", "|", "^", "~",
];

pub fn render_query(f: &mut Frame, area: Rect, data: &ChartData, scroll_offset: usize) {
    // Format the SQL query
    let formatted = sqlformat::format(
        &data.query,
        &sqlformat::QueryParams::None,
        sqlformat::FormatOptions {
            indent: sqlformat::Indent::Spaces(2),
            uppercase: true,
            lines_between_queries: 1,
        },
    );

    let lines: Vec<Line> = formatted
        .lines()
        .enumerate()
        .map(|(i, line)| {
            // Line numbers in gray
            let line_num = format!("{:4} ", i + 1);
            let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];

            // Add syntax-highlighted spans
            spans.extend(highlight_line(line));

            Line::from(spans)
        })
        .collect();

    let total_lines = lines.len();

    // Build title with database name if available
    let title = match &data.database {
        Some(db) => format!(" SQL Query @ {} ({} lines) ", db, total_lines),
        None => format!(" SQL Query ({} lines) ", total_lines),
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));

    f.render_widget(paragraph, area);

    // Render scroll indicator if needed
    if total_lines > area.height as usize - 2 {
        let scroll_info = format!(" {}/{} ", scroll_offset + 1, total_lines);
        let scroll_area = Rect::new(
            area.x + area.width - scroll_info.len() as u16 - 2,
            area.y,
            scroll_info.len() as u16 + 1,
            1,
        );
        let scroll_text =
            Paragraph::new(scroll_info).style(Style::default().fg(Color::DarkGray));
        f.render_widget(scroll_text, scroll_area);
    }
}

/// Highlight a single line of SQL and return colored spans
fn highlight_line(line: &str) -> Vec<Span<'static>> {
    let tokens = tokenize(line);
    tokens
        .into_iter()
        .map(|token| {
            let style = match token.token_type {
                TokenType::Keyword => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                TokenType::Function => Style::default().fg(Color::Blue),
                TokenType::String => Style::default().fg(Color::Green),
                TokenType::Number => Style::default().fg(Color::Yellow),
                TokenType::Comment => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                TokenType::Operator => Style::default().fg(Color::Red),
                TokenType::Punctuation => Style::default().fg(Color::White),
                TokenType::Identifier => Style::default().fg(Color::Cyan),
                TokenType::Whitespace => Style::default(),
            };
            Span::styled(token.text.to_string(), style)
        })
        .collect()
}

/// Tokenize a line of SQL into tokens
fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        // Whitespace
        if ch.is_whitespace() {
            let mut end = start + ch.len_utf8();
            while let Some(&(i, c)) = chars.peek() {
                if c.is_whitespace() {
                    end = i + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                token_type: TokenType::Whitespace,
                text: &input[start..end],
            });
        }
        // Single-line comment
        else if ch == '-' && chars.peek().map(|&(_, c)| c) == Some('-') {
            tokens.push(Token {
                token_type: TokenType::Comment,
                text: &input[start..],
            });
            break; // Rest of line is comment
        }
        // String literal (single quotes)
        else if ch == '\'' {
            let mut end = start + 1;
            let mut escaped = false;
            for (i, c) in chars.by_ref() {
                end = i + c.len_utf8();
                if c == '\'' && !escaped {
                    break;
                }
                escaped = c == '\\';
            }
            tokens.push(Token {
                token_type: TokenType::String,
                text: &input[start..end],
            });
        }
        // Double-quoted identifier
        else if ch == '"' {
            let mut end = start + 1;
            for (i, c) in chars.by_ref() {
                end = i + c.len_utf8();
                if c == '"' {
                    break;
                }
            }
            tokens.push(Token {
                token_type: TokenType::Identifier,
                text: &input[start..end],
            });
        }
        // Number
        else if ch.is_ascii_digit() || (ch == '.' && chars.peek().map(|&(_, c)| c.is_ascii_digit()).unwrap_or(false)) {
            let mut end = start + ch.len_utf8();
            let mut has_dot = ch == '.';
            while let Some(&(i, c)) = chars.peek() {
                if c.is_ascii_digit() {
                    end = i + c.len_utf8();
                    chars.next();
                } else if c == '.' && !has_dot {
                    has_dot = true;
                    end = i + c.len_utf8();
                    chars.next();
                } else if c == 'e' || c == 'E' {
                    end = i + c.len_utf8();
                    chars.next();
                    // Handle optional sign after exponent
                    if let Some(&(i2, c2)) = chars.peek() {
                        if c2 == '+' || c2 == '-' {
                            end = i2 + c2.len_utf8();
                            chars.next();
                        }
                    }
                } else {
                    break;
                }
            }
            tokens.push(Token {
                token_type: TokenType::Number,
                text: &input[start..end],
            });
        }
        // Identifier or keyword
        else if ch.is_alphabetic() || ch == '_' {
            let mut end = start + ch.len_utf8();
            while let Some(&(i, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    end = i + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            let word = &input[start..end];
            let upper = word.to_uppercase();

            let token_type = if SQL_KEYWORDS.contains(&upper.as_str()) {
                TokenType::Keyword
            } else if SQL_FUNCTIONS.contains(&upper.as_str()) {
                TokenType::Function
            } else {
                TokenType::Identifier
            };

            tokens.push(Token {
                token_type,
                text: word,
            });
        }
        // Operators (multi-char)
        else if let Some(op) = match_operator(input, start) {
            // Skip the remaining chars of multi-char operator
            for _ in 1..op.len() {
                chars.next();
            }
            tokens.push(Token {
                token_type: TokenType::Operator,
                text: op,
            });
        }
        // Punctuation
        else if "(),;[]{}".contains(ch) {
            tokens.push(Token {
                token_type: TokenType::Punctuation,
                text: &input[start..start + ch.len_utf8()],
            });
        }
        // Single-char operators
        else if "=<>+-*/%|&^~@#".contains(ch) {
            tokens.push(Token {
                token_type: TokenType::Operator,
                text: &input[start..start + ch.len_utf8()],
            });
        }
        // Unknown - treat as identifier
        else {
            tokens.push(Token {
                token_type: TokenType::Identifier,
                text: &input[start..start + ch.len_utf8()],
            });
        }
    }

    tokens
}

/// Match multi-character operators
fn match_operator(input: &str, start: usize) -> Option<&str> {
    let remaining = &input[start..];

    // Check longer operators first
    for &op in SQL_OPERATORS.iter() {
        if op.len() > 1 && remaining.starts_with(op) {
            return Some(&input[start..start + op.len()]);
        }
    }
    None
}

pub fn get_query_line_count(data: &ChartData) -> usize {
    let formatted = sqlformat::format(
        &data.query,
        &sqlformat::QueryParams::None,
        sqlformat::FormatOptions::default(),
    );
    formatted.lines().count()
}

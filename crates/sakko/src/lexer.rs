use crate::error::{Result, SakkoError};
use crate::span::Span;
use crate::token::{Token, TokenKind};
use std::borrow::Cow;

/// Map a single escape character to its runtime value.
/// Unknown escapes are preserved as `\{esc}` (two characters).
fn handle_escape_sequence(esc: char) -> String {
    match esc {
        'n' => "\n".to_string(),
        't' => "\t".to_string(),
        'r' => "\r".to_string(),
        '"' => "\"".to_string(),
        '\'' => "'".to_string(),
        '`' => "`".to_string(),
        '\\' => "\\".to_string(),
        '$' => "$".to_string(),
        _ => format!("\\{}", esc),
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn char_at(input: &str, i: usize) -> Option<char> {
    input[i..].chars().next()
}

/// Equivalent of the TS check `/\{[\s\S]*?\}/.test(literalContent)`:
/// a `{` exists and some `}` appears after it.
fn has_interpolation(content: &str) -> bool {
    match content.find('{') {
        Some(open) => content[open + 1..].contains('}'),
        None => false,
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token<'_>>> {
    let len = input.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1u32;
    let mut col = 1u32;

    while i < len {
        let ch = char_at(input, i).unwrap();

        if ch == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if ch == '\r' {
            i += 1;
            if input[i..].starts_with('\n') {
                i += 1;
            }
            line += 1;
            col = 1;
            continue;
        }
        if ch == ' ' || ch == '\t' {
            i += ch.len_utf8();
            col += 1;
            continue;
        }

        // Comments: skip to end of line (only if a newline comes before any
        // '<' in the remainder, or the rest of the input has no '<' at all).
        if ch == '/' && input[i + 1..].starts_with('/') {
            let comment_content = &input[i + 2..];
            let next_newline = comment_content.find('\n');
            let next_lt = comment_content.find('<');
            let has_newline_before_lt = match (next_newline, next_lt) {
                (Some(nl), Some(lt)) => nl < lt,
                (Some(_), None) => true,
                (None, _) => false,
            };
            if has_newline_before_lt || next_lt.is_none() {
                while i < len && !input[i..].starts_with('\n') && !input[i..].starts_with('\r') {
                    i += 1;
                }
                continue;
            }
            // '<' comes before the newline: don't treat as a comment.
        }

        if let Some(kind) = symbol_kind(ch) {
            let start = i;
            let value = kind.symbol_str().unwrap();
            tokens.push(Token {
                kind,
                value: Cow::Borrowed(value),
                span: Span::new(start, start + 1),
                line,
                col,
            });
            i += 1;
            col += 1;
            continue;
        }

        if ch == '"' || ch == '`' {
            let quote = ch;
            let start_col = col;
            let start_byte = i;
            i += 1; // past opening quote
            col += 1;

            // Scan up to the next unescaped closing quote so we don't
            // mistake braces outside this literal for interpolation ends.
            let mut scan_end = i;
            while scan_end < len && !input[scan_end..].starts_with(quote) {
                if input[scan_end..].starts_with('\\') && scan_end + 1 < len {
                    scan_end += 2;
                } else {
                    scan_end += 1;
                }
            }
            let literal_content = &input[i..scan_end];

            if quote == '"' && has_interpolation(literal_content) {
                let result = tokenize_string_with_interpolation(input, i, line, col, start_col)?;
                tokens.extend(result.tokens);
                i = result.end_index + 1;
                line = result.end_line;
                col = result.end_col + 1;
                continue;
            }

            let mut owned: Option<String> = None;
            while i < len && !input[i..].starts_with(quote) {
                if input[i..].starts_with('\\') && i + 1 < len {
                    let dst = owned.get_or_insert_with(|| String::from(&input[start_byte + 1..i]));
                    let esc = char_at(input, i + 1).unwrap();
                    dst.push_str(&handle_escape_sequence(esc));
                    i += 1 + esc.len_utf8();
                    col += 2;
                    continue;
                }
                let c = char_at(input, i).unwrap();
                if let Some(dst) = owned.as_mut() {
                    dst.push(c);
                }
                if c == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += c.len_utf8();
            }
            if i >= len {
                return Err(SakkoError::new(format!(
                    "Unterminated string at line {}, col {}",
                    line, start_col
                ))
                .with_suggestion(format!("Add a closing {}", quote)));
            }
            i += 1; // closing quote
            col += 1;
            let kind = if quote == '`' {
                TokenKind::BacktickString
            } else {
                TokenKind::String
            };
            let value = match owned {
                Some(s) => Cow::Owned(s),
                None => Cow::Borrowed(&input[start_byte + 1..i - 1]),
            };
            tokens.push(Token {
                kind,
                value,
                span: Span::new(start_byte, i),
                line,
                col: start_col,
            });
            continue;
        }

        if is_ident_char(ch) {
            let start = i;
            let start_col = col;
            while i < len && char_at(input, i).is_some_and(is_ident_char) {
                i += char_at(input, i).unwrap().len_utf8();
                col += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Ident,
                value: Cow::Borrowed(&input[start..i]),
                span: Span::new(start, i),
                line,
                col: start_col,
            });
            continue;
        }

        return Err(SakkoError::new(format!(
            "Unexpected character: {} at line {}, col {}",
            ch, line, col
        ))
        .with_suggestion("Remove or escape this character"));
    }

    Ok(tokens)
}

fn symbol_kind(c: char) -> Option<TokenKind> {
    Some(match c {
        '<' => TokenKind::Lt,
        '>' => TokenKind::Gt,
        '{' => TokenKind::Lbrace,
        '}' => TokenKind::Rbrace,
        '(' => TokenKind::Lparen,
        ')' => TokenKind::Rparen,
        '[' => TokenKind::Lbracket,
        ']' => TokenKind::Rbracket,
        ':' => TokenKind::Colon,
        ';' => TokenKind::Semi,
        ',' => TokenKind::Comma,
        '@' => TokenKind::At,
        '=' => TokenKind::Equals,
        '.' => TokenKind::Dot,
        '+' => TokenKind::Plus,
        '-' => TokenKind::Minus,
        '*' => TokenKind::Star,
        '|' => TokenKind::Pipe,
        '&' => TokenKind::Ampersand,
        '!' => TokenKind::Bang,
        '?' => TokenKind::Question,
        '%' => TokenKind::Percent,
        _ => return None,
    })
}

struct InterpolationResult<'a> {
    tokens: Vec<Token<'a>>,
    /// Index of the closing quote.
    end_index: usize,
    end_line: u32,
    end_col: u32,
}

fn tokenize_string_with_interpolation(
    input: &str,
    start_index: usize,
    line: u32,
    col: u32,
    original_start_col: u32,
) -> Result<InterpolationResult<'_>> {
    let len = input.len();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = start_index;
    let mut current_line = line;
    let mut current_col = col;
    let mut text_buffer = String::new();
    let mut text_start_col = current_col;
    // Byte position where the current run of text characters began.
    let mut text_part_start = i;

    while i < len && !input[i..].starts_with('"') {
        if input[i..].starts_with('{') {
            if !text_buffer.is_empty() {
                tokens.push(Token {
                    kind: TokenKind::String,
                    value: Cow::Owned(std::mem::take(&mut text_buffer)),
                    span: Span::new(text_part_start, i),
                    line: current_line,
                    col: text_start_col,
                });
            }

            tokens.push(Token {
                kind: TokenKind::InterpStart,
                value: Cow::Borrowed("{"),
                span: Span::new(i, i + 1),
                line: current_line,
                col: current_col,
            });
            i += 1;
            current_col += 1;

            let mut expr = String::new();
            let mut brace_depth = 1u32;
            let expr_start = i;
            let expr_start_col = current_col;

            while i < len && brace_depth > 0 {
                if input[i..].starts_with('{') {
                    brace_depth += 1;
                }
                if input[i..].starts_with('}') {
                    brace_depth -= 1;
                }
                if brace_depth > 0 {
                    expr.push(char_at(input, i).unwrap());
                }
                if input[i..].starts_with('\n') {
                    current_line += 1;
                    current_col = 1;
                } else {
                    current_col += 1;
                }
                i += char_at(input, i).unwrap().len_utf8();
            }

            if brace_depth > 0 {
                return Err(SakkoError::new(format!(
                    "Unterminated interpolation expression at line {}, col {}",
                    current_line, expr_start_col
                ))
                .with_suggestion("Add a closing brace '}'"));
            }

            tokens.push(Token {
                kind: TokenKind::Expr,
                value: Cow::Owned(expr.trim().to_string()),
                span: Span::new(expr_start, i - 1),
                line: current_line,
                col: expr_start_col,
            });
            tokens.push(Token {
                kind: TokenKind::InterpEnd,
                value: Cow::Borrowed("}"),
                span: Span::new(i - 1, i),
                line: current_line,
                col: current_col - 1,
            });

            text_start_col = current_col;
            text_part_start = i;
            continue;
        }

        // Escape sequences inside interpolated strings.
        if input[i..].starts_with('\\') && i + 1 < len {
            if text_buffer.is_empty() {
                text_part_start = i;
            }
            i += 1;
            current_col += 1;
            let esc = char_at(input, i).unwrap();
            text_buffer.push_str(&handle_escape_sequence(esc));
            current_col += 1;
            i += esc.len_utf8();
            continue;
        }

        let c = char_at(input, i).unwrap();
        if text_buffer.is_empty() {
            text_part_start = i;
        }
        text_buffer.push(c);
        if c == '\n' {
            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }
        i += c.len_utf8();
    }

    if i >= len {
        return Err(SakkoError::new(format!(
            "Unterminated string at line {}, col {}",
            current_line, original_start_col
        ))
        .with_suggestion("Add a closing quote \""));
    }

    if !text_buffer.is_empty() || tokens.is_empty() {
        tokens.push(Token {
            kind: TokenKind::String,
            value: Cow::Owned(text_buffer),
            span: Span::new(0, 0),
            line: current_line,
            col: text_start_col,
        });
    }

    Ok(InterpolationResult {
        tokens,
        end_index: i,
        end_line: current_line,
        end_col: current_col,
    })
}

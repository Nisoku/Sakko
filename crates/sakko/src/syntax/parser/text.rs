//! Snippet reconstruction: rebuilding source text from token streams for
//! block bodies (`@effect`) and single expressions (state initializers).

use crate::error::Result;
use crate::syntax::token::{Token, TokenKind};
use std::borrow::Cow;

use super::Parser;

impl<'a> Parser<'a> {
    fn should_insert_space(current: &str, next: &Token) -> bool {
        if current.is_empty() {
            return false;
        }
        let Some(last_char) = current.chars().last() else {
            return false;
        };
        let next_char = match next.value.chars().next() {
            Some(c) => c,
            None => return false,
        };
        let is_word_end = is_js_word_char(last_char);
        let is_word_start = is_js_word_char(next_char);
        is_word_end && is_word_start
    }

    pub fn parse_block_body(&mut self) -> Result<Cow<'a, str>> {
        let mut body = String::new();
        let mut brace_depth = 0i32;
        let mut prev_line: Option<u32> = None;

        while let Some(token) = self.peek() {
            if token.kind == TokenKind::Rbrace && brace_depth == 0 {
                break;
            }

            if token.kind == TokenKind::Lbrace {
                brace_depth += 1;
            }
            if token.kind == TokenKind::Rbrace {
                brace_depth -= 1;
            }

            let on_new_line = matches!(prev_line, Some(pl) if pl < token.line);
            if on_new_line {
                body.push('\n');
            } else if Self::should_insert_space(&body, token) {
                body.push(' ');
            }

            match token.kind {
                TokenKind::String => {
                    body.push_str(&json_quote(&token.value));
                }
                TokenKind::BacktickString => {
                    body.push('`');
                    body.push_str(&token.value);
                    body.push('`');
                }
                _ => body.push_str(&token.value),
            }
            prev_line = Some(token.line);
            self.consume()?;
        }

        Ok(Cow::Owned(body.trim().to_string()))
    }

    pub fn parse_expression(&mut self) -> Result<Cow<'a, str>> {
        let mut expr = String::new();
        let mut paren_depth = 0i32;
        let mut brace_depth = 0i32;
        let mut bracket_depth = 0i32;

        while let Some(token) = self.peek() {
            if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                if token.kind == TokenKind::Semi
                    || token.kind == TokenKind::Rbrace
                    || token.kind == TokenKind::Comma
                {
                    break;
                }
                if token.kind == TokenKind::Ident
                    && self.peek_ahead_is(TokenKind::Equals)
                    && self.peek_ahead(2).map(|t| t.kind) != Some(TokenKind::Equals)
                {
                    break;
                }
            }

            match token.kind {
                TokenKind::Lparen => paren_depth += 1,
                TokenKind::Rparen => paren_depth -= 1,
                TokenKind::Lbrace => brace_depth += 1,
                TokenKind::Rbrace => brace_depth -= 1,
                TokenKind::Lbracket => bracket_depth += 1,
                TokenKind::Rbracket => bracket_depth -= 1,
                _ => {}
            }

            if Self::should_insert_space(&expr, token) {
                expr.push(' ');
            }
            if token.kind == TokenKind::String {
                expr.push_str(&json_quote(&token.value));
            } else {
                expr.push_str(&token.value);
            }
            self.consume()?;
        }

        Ok(Cow::Owned(expr.trim().to_string()))
    }
}

fn is_js_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// Reproduce `JSON.stringify(str)` output: quotes plus standard escapes.
pub(crate) fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

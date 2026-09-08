//! Snippet reconstruction: rebuilding source text from token streams, then
//! parsing it once into a typed Saho snippet for the document IR. Both the
//! exact rebuilt source and the pre-parsed form are retained.

use crate::error::Result;
use crate::syntax::ast::{BlockSnippet, ExprSnippet};
use crate::syntax::token::{Token, TokenKind};
use std::borrow::Cow;

use super::Parser;

impl<'a> Parser<'a> {
    /// Reconstruct and pre-parse a block body (`@effect`, `@on`), producing a
    /// typed snippet. Called once at parse time; the typechecker never re-parses.
    pub fn parse_block_snippet(&mut self) -> Result<BlockSnippet<'a>> {
        Ok(BlockSnippet::parse(self.parse_block_body()?))
    }

    /// Reconstruct and pre-parse an expression, producing a typed snippet.
    pub fn parse_expr_snippet(&mut self) -> Result<ExprSnippet<'a>> {
        Ok(ExprSnippet::parse(self.parse_expression()?))
    }

    /// Reconstruct an expression and require that it is a single identifier
    /// (used for `@each`'s item binding). Returns the identifier source.
    pub fn parse_each_clause(&mut self) -> Result<(Cow<'a, str>, ExprSnippet<'a>)> {
        // `@each="item in source"` string form.
        if self.check(TokenKind::Equals) && self.peek_ahead_is(TokenKind::String) {
            self.consume()?; // consume =
            let spec = self.consume()?.value;
            let Some(mid) = spec.find(" in ") else {
                let err = self.error_at(
                    "Expected 'item in source' in @each expression",
                    self.last_token(),
                );
                return Err(err);
            };
            let item = Cow::Owned(spec[..mid].to_string());
            let source = ExprSnippet::parse(Cow::Owned(spec[mid + 4..].to_string()));
            return Ok((item, source));
        }

        let item = self.expect(TokenKind::Ident, None)?.value;
        let in_token = self.expect(TokenKind::Ident, Some("Expected 'in' in @each expression"))?;
        if in_token.value != "in" {
            let err = self.error_at(
                format!(
                    "Expected 'in' in @each expression, got '{}'",
                    in_token.value
                ),
                Some(&in_token),
            );
            return Err(err);
        }
        let source = self.parse_expr_snippet()?;
        Ok((item, source))
    }
}

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
        self.parse_expression_until(|_, _| false)
    }

    /// Like [`Self::parse_expression`] but stops the scan early whenever
    /// `extra_stop` returns true at depth 0, in addition to the built-in
    /// terminators (`;`, `)`, `}`, `,`). Modifier expression args use this to
    /// end an expression at a depth-0 identifier that cannot continue it
    /// (`a ? b : c large` stops before `large`, which is a separate flag).
    pub fn parse_expression_until(
        &mut self,
        mut extra_stop: impl FnMut(&Token<'a>, Option<TokenKind>) -> bool,
    ) -> Result<Cow<'a, str>> {
        let mut expr = String::new();
        let mut paren_depth = 0i32;
        let mut brace_depth = 0i32;
        let mut bracket_depth = 0i32;
        let mut prev_kind: Option<TokenKind> = None;

        while let Some(token) = self.peek() {
            if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                if token.kind == TokenKind::Semi
                    || token.kind == TokenKind::Rparen
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
                if extra_stop(token, prev_kind) {
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

            // An interpolated string was lexed into separate `String`,
            // `InterpStart`, `Expr`, `InterpEnd` tokens. Reassemble them into a
            // single Saho `"..."` string with `{expr}` interpolation so the
            // Saho lexer (which supports inline `{expr}` in double quotes)
            // sees the original form again.
            if token.kind == TokenKind::String && self.peek_ahead_is(TokenKind::InterpStart) {
                if Self::should_insert_space(&expr, token) {
                    expr.push(' ');
                }
                expr.push_str(&self.reassemble_interpolated_string()?);
                prev_kind = Some(TokenKind::String);
                continue;
            }

            if Self::should_insert_space(&expr, token) {
                expr.push(' ');
            }
            if token.kind == TokenKind::String {
                expr.push_str(&json_quote(&token.value));
            } else {
                expr.push_str(&token.value);
            }
            prev_kind = Some(token.kind);
            self.consume()?;
        }

        Ok(Cow::Owned(expr.trim().to_string()))
    }

    /// Reassemble the tokens of an interpolated string (`String`,
    /// `InterpStart`, `Expr`, `InterpEnd`) into a single Saho `"..."` string
    /// with `{expr}` interpolation. Consumes the full sequence.
    fn reassemble_interpolated_string(&mut self) -> Result<Cow<'a, str>> {
        let mut out = String::new();
        out.push('"');
        loop {
            if !self.peek().is_some_and(|t| matches_token_in_interp(t.kind)) {
                break;
            }
            let token = self.consume()?;
            match token.kind {
                TokenKind::String => out.push_str(&json_quote_body(&token.value)),
                TokenKind::InterpStart => out.push_str(&token.value),
                TokenKind::Expr => out.push_str(&token.value),
                TokenKind::InterpEnd => out.push_str(&token.value),
                _ => break,
            }
            if token.kind == TokenKind::InterpEnd {
                break;
            }
        }
        out.push('"');
        Ok(Cow::Owned(out.to_string()))
    }
}

fn matches_token_in_interp(kind: TokenKind) -> bool {
    kind == TokenKind::String
        || kind == TokenKind::InterpStart
        || kind == TokenKind::Expr
        || kind == TokenKind::InterpEnd
}

fn is_js_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// Escape a string body for embedding inside a Saho `"..."` literal. Braces
/// are escaped so a literal `{`/`}` is not mistaken for interpolation.
fn json_quote_body(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
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
    out
}

/// Reproduce `JSON.stringify(str)` output: quotes plus standard escapes.
pub(crate) fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    out.push_str(&json_quote_body(s));
    out.push('"');
    out
}

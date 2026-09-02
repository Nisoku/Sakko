//! Structure parser for Sakko documents.

mod atcodes;
mod elements;
mod modifiers;
mod text;

use crate::error::{Result, SakkoError};
use crate::syntax::ast::RootNode;
use crate::syntax::lexer::tokenize;
use crate::syntax::token::{Token, TokenKind};
use std::borrow::Cow;

pub struct Parser<'a> {
    pub tokens: Vec<Token<'a>>,
    pub position: usize,
    source_len: usize,
    lines: Vec<Cow<'a, str>>,
}

impl<'a> Parser<'a> {
    /// `lines` supplies the snippet text per line. The normal path passes
    /// borrowed lines of the source; the auto-wrap path passes a virtual
    /// wrapper layout (see `parse_sakko`).
    pub fn with_lines(tokens: Vec<Token<'a>>, lines: Vec<Cow<'a, str>>) -> Self {
        let source_len = lines
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            .saturating_sub(1);
        Self {
            tokens,
            position: 0,
            source_len,
            lines,
        }
    }

    pub fn new(tokens: Vec<Token<'a>>, source: &'a str) -> Self {
        let lines: Vec<Cow<'a, str>> = source.split('\n').map(Cow::Borrowed).collect();
        Self::with_lines(tokens, lines)
    }

    /// Build an error
    pub fn error_at(&self, msg: impl Into<String>, token: Option<&Token>) -> SakkoError {
        let msg = msg.into();
        let mut err = SakkoError::new(msg.clone());
        err.suggestion = self.get_suggestion(&msg);
        let token = match token {
            Some(t) => t,
            None => return err,
        };
        if self.source_len == 0 {
            return err;
        }
        let line_text = self
            .lines
            .get(token.line as usize - 1)
            .map(Cow::as_ref)
            .unwrap_or("");
        let pointer = format!("{}^", " ".repeat(token.col.saturating_sub(1) as usize));
        err.line = Some(token.line);
        err.col = Some(token.col);
        err.snippet = Some(format!(
            "\n  {}\n  {}",
            line_text.trim_end_matches('\r'),
            pointer
        ));
        err
    }

    fn get_suggestion(&self, msg: &str) -> Option<String> {
        if msg.contains("Unexpected end of input") {
            Some("Check for missing closing brackets".to_string())
        } else if msg.contains("Expected") {
            Some("Add the expected token".to_string())
        } else if msg.contains("Unexpected token") {
            Some("Remove or replace this token".to_string())
        } else {
            None
        }
    }

    pub(crate) fn last_token(&self) -> Option<&Token<'a>> {
        self.tokens.last()
    }

    pub fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.position)
    }

    pub fn peek_ahead(&self, offset: usize) -> Option<&Token<'a>> {
        self.tokens.get(self.position + offset)
    }

    pub fn peek_ahead_is(&self, kind: TokenKind) -> bool {
        self.peek_ahead(1).is_some_and(|t| t.kind == kind)
    }

    pub fn consume(&mut self) -> Result<Token<'a>> {
        let token = self.peek().cloned();
        match token {
            Some(t) => {
                self.position += 1;
                Ok(t)
            }
            None => {
                let err = self.error_at("Unexpected end of input", self.last_token());
                Err(err)
            }
        }
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|t| t.kind == kind)
    }

    pub fn expect(&mut self, kind: TokenKind, error_msg: Option<&str>) -> Result<Token<'a>> {
        let mismatch = match self.peek() {
            Some(t) if t.kind == kind => None,
            other => Some(match error_msg {
                Some(m) => m.to_string(),
                None => format!(
                    "Expected {} but got {}",
                    kind.ts_name(),
                    other.map(|t| t.kind.ts_name()).unwrap_or("end of input")
                ),
            }),
        };
        match mismatch {
            Some(msg) => Err(self.error_at(msg, self.peek())),
            None => self.consume(),
        }
    }

    pub fn parse_root(&mut self) -> Result<RootNode<'a>> {
        self.expect(TokenKind::Lt, Some("Expected '<'"))?;

        let name_token = self.peek();
        if name_token.map(|t| t.kind) != Some(TokenKind::Ident) {
            let err = self.error_at("Expected identifier after '<'", name_token);
            return Err(err);
        }
        let name = self.consume()?.value;

        let modifiers = if self.check(TokenKind::Lparen) {
            self.parse_modifiers()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::Lbrace, Some("Expected '{'"))?;

        let mut declarations = Vec::new();
        let mut children = Vec::new();

        while !self.check(TokenKind::Rbrace) {
            if self.peek().is_none() {
                let err = self.error_at("Unexpected end of input, expected '}'", self.last_token());
                return Err(err);
            }

            if self.check(TokenKind::At) {
                let at_token = self.consume()?;
                declarations.push(self.parse_atcode_declaration(&at_token)?);
            } else {
                children.push(self.parse_node()?);
            }

            if self.check(TokenKind::Semi) || self.check(TokenKind::Comma) {
                self.consume()?;
            }
        }

        self.expect(TokenKind::Rbrace, Some("Expected '}'"))?;
        self.expect(TokenKind::Gt, Some("Expected '>'"))?;

        Ok(RootNode {
            name,
            modifiers,
            declarations,
            children,
        })
    }
}

/// Parse a Sakko template. Input lacking an explicit `<root { ... }>` wrapper
/// is parsed as a component body using the reserved sentinel wrapper name.
pub fn parse_sakko<'a>(input: &'a str) -> Result<RootNode<'a>> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(SakkoError::new("Empty input").with_suggestion("Add some content to parse"));
    }

    let mut tokens = tokenize(input)?;

    let (lines, tokens) = if !trimmed.starts_with('<') {
        let start_byte = input.len() - input.trim_start().len();
        let prefix = &input[..start_byte];
        let leading_lines = prefix.matches('\n').count() as u32;
        // Whitespace chars between the last newline of the prefix and the
        // first non-whitespace char of the trimmed content.
        let first_line_ws = match prefix.rfind('\n') {
            Some(nl) => start_byte - nl - 1,
            None => start_byte,
        } as u32;
        let first_content_line = leading_lines + 1;

        for t in tokens.iter_mut() {
            if t.line == first_content_line {
                t.col = t.col.saturating_sub(first_line_ws).max(1);
            }
            // Drop the stripped leading lines, then add the wrapper header.
            t.line = t.line.saturating_sub(leading_lines).saturating_add(1);
        }

        // Virtual layout: header line 1, trimmed content from line 2.
        let mut lines: Vec<Cow<'a, str>> = Vec::with_capacity(trimmed.split('\n').count() + 2);
        lines.push(Cow::Borrowed("<__sakko_wrapper__ {"));
        lines.extend(trimmed.split('\n').map(Cow::Borrowed));
        lines.push(Cow::Borrowed("}>"));

        let last_line = tokens.last().map(|t| t.line).unwrap_or(2);
        let synthetic = |kind: TokenKind, value: &'static str, col: u32| Token {
            kind,
            value: Cow::Borrowed(value),
            span: Default::default(),
            line: last_line + 1,
            col,
        };
        let mut wrapped = Vec::with_capacity(tokens.len() + 5);
        wrapped.push(Token {
            kind: TokenKind::Lt,
            value: Cow::Borrowed("<"),
            span: Default::default(),
            line: 1,
            col: 1,
        });
        wrapped.push(Token {
            kind: TokenKind::Ident,
            value: Cow::Borrowed(crate::WRAPPER_NAME),
            span: Default::default(),
            line: 1,
            col: 2,
        });
        wrapped.push(Token {
            kind: TokenKind::Lbrace,
            value: Cow::Borrowed("{"),
            span: Default::default(),
            // "<" + "__sakko_wrapper__" + " " = 1 + 17 + 1 chars before '{'
            line: 1,
            col: 2 + crate::WRAPPER_NAME.len() as u32 + 1,
        });
        wrapped.append(&mut tokens);
        wrapped.push(synthetic(TokenKind::Rbrace, "}", 1));
        wrapped.push(synthetic(TokenKind::Gt, ">", 2));
        (lines, wrapped)
    } else {
        let lines: Vec<Cow<'a, str>> = input.split('\n').map(Cow::Borrowed).collect();
        (lines, tokens)
    };

    let mut parser = Parser::with_lines(tokens, lines);
    parser.parse_root()
}

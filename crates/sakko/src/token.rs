//! Token kinds and tokens produced by the lexer.

use crate::span::Span;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")] // Lol
pub enum TokenKind {
    Lt,
    Gt,
    Lbrace,
    Rbrace,
    Lparen,
    Rparen,
    Lbracket,
    Rbracket,
    Colon,
    Semi,
    Comma,
    Ident,
    String,
    BacktickString,
    At,
    Equals,
    InterpStart,
    InterpEnd,
    Expr,
    Dot,
    Plus,
    Minus,
    Star,
    Pipe,
    Ampersand,
    Bang,
    Question,
    Percent,
}

impl TokenKind {
    /// The exact `TokenType` string spelling used by the TS implementation.
    /// Appears verbatim in default parser error messages
    /// (``Expected ${type} but got ${...}``).
    pub fn ts_name(self) -> &'static str {
        match self {
            TokenKind::Lt => "LT",
            TokenKind::Gt => "GT",
            TokenKind::Lbrace => "LBRACE",
            TokenKind::Rbrace => "RBRACE",
            TokenKind::Lparen => "LPAREN",
            TokenKind::Rparen => "RPAREN",
            TokenKind::Lbracket => "LBRACKET",
            TokenKind::Rbracket => "RBRACKET",
            TokenKind::Colon => "COLON",
            TokenKind::Semi => "SEMI",
            TokenKind::Comma => "COMMA",
            TokenKind::Ident => "IDENT",
            TokenKind::String => "STRING",
            TokenKind::BacktickString => "BACKTICK_STRING",
            TokenKind::At => "AT",
            TokenKind::Equals => "EQUALS",
            TokenKind::InterpStart => "INTERP_START",
            TokenKind::InterpEnd => "INTERP_END",
            TokenKind::Expr => "EXPR",
            TokenKind::Dot => "DOT",
            TokenKind::Plus => "PLUS",
            TokenKind::Minus => "MINUS",
            TokenKind::Star => "STAR",
            TokenKind::Pipe => "PIPE",
            TokenKind::Ampersand => "AMPERSAND",
            TokenKind::Bang => "BANG",
            TokenKind::Question => "QUESTION",
            TokenKind::Percent => "PERCENT",
        }
    }

    /// The canonical single-character spelling used by the TS tokenizer's
    /// `value` field for symbol tokens.
    pub fn symbol_str(self) -> Option<&'static str> {
        Some(match self {
            TokenKind::Lt => "<",
            TokenKind::Gt => ">",
            TokenKind::Lbrace => "{",
            TokenKind::Rbrace => "}",
            TokenKind::Lparen => "(",
            TokenKind::Rparen => ")",
            TokenKind::Lbracket => "[",
            TokenKind::Rbracket => "]",
            TokenKind::Colon => ":",
            TokenKind::Semi => ";",
            TokenKind::Comma => ",",
            TokenKind::At => "@",
            TokenKind::Equals => "=",
            TokenKind::Dot => ".",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Pipe => "|",
            TokenKind::Ampersand => "&",
            TokenKind::Bang => "!",
            TokenKind::Question => "?",
            TokenKind::Percent => "%",
            TokenKind::Ident
            | TokenKind::String
            | TokenKind::BacktickString
            | TokenKind::InterpStart
            | TokenKind::InterpEnd
            | TokenKind::Expr => return None,
        })
    }
}

/// A lexed token. `value` borrows from the source input unless escape
/// decoding or interpolation assembly required an owned copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token<'a> {
    pub kind: TokenKind,
    #[serde(borrow)]
    pub value: Cow<'a, str>,
    pub span: Span,
    pub line: u32,
    pub col: u32,
}

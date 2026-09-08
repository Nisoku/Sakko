//! Error types shared by the lexer and parser.

use std::fmt;

/// A Sakko lexing/parsing error with optional position info, source snippet
/// with caret pointer, and a fix suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SakkoError {
    pub message: String,
    /// 1-based line/column, when known.
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub suggestion: Option<String>,
    /// Pre-rendered `\n  {lineText}\n  {pointer}` block
    pub snippet: Option<String>,
}

impl SakkoError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            col: None,
            suggestion: None,
            snippet: None,
        }
    }

    pub fn with_position(mut self, line: u32, col: u32) -> Self {
        self.line = Some(line);
        self.col = Some(col);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    pub fn tokenizer(message: &str, line: u32, col: u32, suggestion: Option<&str>) -> Self {
        let mut err = Self::new(message.to_string()).with_position(line, col);
        if let Some(s) = suggestion {
            err = err.with_suggestion(s);
        }
        err
    }
}

impl fmt::Display for SakkoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.col) {
            (Some(l), Some(c)) => write!(f, "{} at line {}, col {}", self.message, l, c)?,
            _ => write!(f, "{}", self.message)?,
        }
        if let Some(snippet) = &self.snippet {
            write!(f, "{}", snippet)?;
        }
        Ok(())
    }
}

impl std::error::Error for SakkoError {}

pub type Result<T> = std::result::Result<T, SakkoError>;

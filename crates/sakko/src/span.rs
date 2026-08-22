//! Byte spans and line/column resolution.

use serde::{Deserialize, Serialize};

/// Half-open byte range `[start, end)` into the source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }
}

/// Precomputed newline positions for fast `byte offset -> (line, col)` lookups.
pub struct LineIndex {
    /// Byte offset of the first character of each line.
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// 1-based `(line, col)` for a byte offset. Column counts bytes.
    pub fn line_col(&self, pos: usize) -> (u32, u32) {
        let line = match self.line_starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let col = pos - self.line_starts[line] + 1;
        (line as u32 + 1, col as u32)
    }
}

//! Serializable DTOs for the WASM bridge.

use sakko::span::Span;
use serde::Serialize;

/// A document-level lex/parse failure, as returned by the host functions
/// (`tokenize` / `parse` / `check`) when the source cannot be parsed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDto {
    pub message: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub suggestion: Option<String>,
    pub snippet: Option<String>,
}

impl From<&sakko::SakkoError> for ErrorDto {
    fn from(e: &sakko::SakkoError) -> Self {
        Self {
            message: e.message.clone(),
            line: e.line,
            col: e.col,
            suggestion: e.suggestion.clone(),
            snippet: e.snippet.clone(),
        }
    }
}

/// A single typecheck diagnostic.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagDto {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub kind_label: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    /// Byte offsets into `snippet`.
    pub span: Span,
    pub snippet: String,
    /// The compiler-grade text rendering (`[SKT000]: ...` + source caret).
    pub rendered: String,
}

impl From<&sakko::typecheck::Diagnostic> for DiagDto {
    fn from(d: &sakko::typecheck::Diagnostic) -> Self {
        Self {
            severity: d.severity.to_string(),
            code: d.code.id().to_string(),
            message: d.message.clone(),
            kind_label: d.kind_label.clone(),
            line: d.location.map(|(l, _)| l),
            col: d.location.map(|(_, c)| c),
            span: d.span,
            snippet: d.snippet.clone(),
            rendered: d.render(),
        }
    }
}

/// One recorded `js { ... }` escape.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsEscapeDto {
    pub kind_label: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub span: Span,
    pub body: String,
}

impl From<&sakko::typecheck::JsEscape> for JsEscapeDto {
    fn from(e: &sakko::typecheck::JsEscape) -> Self {
        Self {
            kind_label: e.kind_label.clone(),
            line: e.location.map(|(l, _)| l),
            col: e.location.map(|(_, c)| c),
            span: e.span,
            body: e.body.clone(),
        }
    }
}

/// Everything a typecheck run reports, in a stable JSON shape.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub ok: bool,
    pub diagnostics: Vec<DiagDto>,
    pub js_escapes: Vec<JsEscapeDto>,
}

impl From<&sakko::typecheck::Report> for CheckReport {
    fn from(r: &sakko::typecheck::Report) -> Self {
        Self {
            ok: r.diagnostics.is_empty(),
            diagnostics: r.diagnostics.iter().map(DiagDto::from).collect(),
            js_escapes: r.js_escapes.iter().map(JsEscapeDto::from).collect(),
        }
    }
}

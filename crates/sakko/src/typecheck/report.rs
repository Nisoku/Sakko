//! Compile-report output types: diagnostics plus raw-JS escape records.

use super::diag::Diagnostic;

/// One recorded `js { ... }` occurrence. Sakko never checks these bodies
/// semantically
#[derive(Debug, Clone)]
pub struct JsEscape {
    pub kind_label: String,
    pub location: Option<(u32, u32)>,
    /// Byte offsets of the whole block within [`Diagnostic::snippet`].
    pub span: crate::span::Span,
    /// Raw JavaScript source between the braces.
    pub body: String,
}

/// Everything a typecheck run observed.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub js_escapes: Vec<JsEscape>,
}

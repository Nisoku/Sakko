use crate::span::Span;
use std::fmt;
use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
        }
    }
}

macro_rules! codes {
    ($($variant:ident => $id:literal),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Code {
            $($variant,)*
        }

        impl Code {
            pub fn id(self) -> &'static str {
                match self {
                    $(Code::$variant => $id,)*
                }
            }
        }
    };
}

codes! {
    UnknownIdent => "SKT001",
    UnknownProp => "SKT002",
    NotCallable => "SKT003",
    AssignMismatch => "SKT004",
    BadOperand => "SKT005",
    BadUnaryOperand => "SKT006",
    DuplicateDecl => "SKT007",
    BadBindTarget => "SKT008",
    BadEachSource => "SKT009",
    RenderedFunction => "SKT010",
    SnippetParse => "SKT011",
    ConstReassign => "SKT012",
    UnknownUse => "SKT013",
    ImpossibleCast => "SKT014",
    BadClassType => "SKT015",
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Code,
    pub message: String,
    /// Human label of the checked snippet, e.g. `@state 'count'` or `@on:click`.
    pub kind_label: String,
    /// Document position of the enclosing declaration when known.
    pub location: Option<(u32, u32)>,
    /// Byte offsets into `snippet`.
    pub span: Span,
    pub snippet: String,
}

impl Diagnostic {
    pub fn render(&self) -> String {
        let start = self.span.start as usize;
        let end = (self.span.end as usize).clamp(start, self.snippet.len());

        let line_start = self.snippet[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = self.snippet[line_start..]
            .find('\n')
            .map_or(self.snippet.len(), |i| line_start + i);
        let text = &self.snippet[line_start..line_end];

        let line_no = self.snippet[..line_start].matches('\n').count() + 1;
        let gutter = line_no.to_string();
        let pad = " ".repeat(gutter.len());
        let col = self.snippet[line_start..start].chars().count();
        let line_len = text.chars().count();
        let caret_len = self.snippet[start..end]
            .chars()
            .count()
            .max(1)
            .min((line_len - col.min(line_len)).max(1));
        let caret = "^".repeat(caret_len);

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}[{}]: {}",
            self.severity,
            self.code.id(),
            self.message
        );
        match self.location {
            Some((line, doc_col)) => {
                let _ = write!(
                    out,
                    "  --> {} at line {}, col {}",
                    self.kind_label, line, doc_col
                );
            }
            None => {
                let _ = write!(out, "  --> {}", self.kind_label);
            }
        }
        out.push('\n');
        let _ = writeln!(out, " {pad} |");
        let _ = writeln!(out, " {gutter} | {text}");
        let _ = writeln!(out, " {pad} | {}{caret}", " ".repeat(col));
        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

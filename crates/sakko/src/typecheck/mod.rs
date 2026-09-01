//! Typechecker for Sakko documents: inference over every snippet with
//! stable diagnostic codes and a compile [`Report`].

pub mod builtins;
pub mod checker;
pub mod diag;
pub mod driver;
pub mod report;
pub mod types;

pub use diag::{Code, Diagnostic, Severity};
pub use driver::{check_ast, check_source};
pub use report::{JsEscape, Report};
pub use types::Ty;

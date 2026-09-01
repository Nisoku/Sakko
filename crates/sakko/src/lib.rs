//! # sakko
//!
//! Lexer, parser, AST, and typechecker for the Sakko template language.
//!
//! - [`syntax`] - the document language (`<name { ... }>`): tokens,
//!   structure parser, AST.
//! - [`saho`] - the embedded strict expression language used inside
//!   snippets and interpolations.
//! - [`typecheck`] - inference and diagnostics over the combined AST.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod error;
pub mod saho;
pub mod span;
pub mod syntax;
pub mod typecheck;

pub use error::{Result, SakkoError};
pub use span::{LineIndex, Span};
pub use syntax::ast::{
    AstNode, AtcodeDeclaration, DerivedVar, ElementNode, InlineNode, InlineValue, InterpolatedText,
    InterpolatedTextPart, ListNode, Modifier, RootNode, StateVar,
};
pub use syntax::lexer::tokenize;
pub use syntax::parser::{Parser, parse_sakko};
pub use syntax::token::{Token, TokenKind};
pub use typecheck::{Diagnostic, JsEscape, Report, check_ast, check_source};

/// Reserved sentinel name used when auto-wrapping input that lacks an
/// explicit root (`<name { ... }>`).
pub const WRAPPER_NAME: &str = "__sakko_wrapper__";

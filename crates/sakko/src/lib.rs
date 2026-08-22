//! # sakko
//!
//! Lexer, parser, and AST for the Sakko template language.

pub mod ast;
pub mod error;
pub mod expr;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;
pub mod typecheck;

pub use ast::{
    AstNode, AtcodeDeclaration, DerivedVar, ElementNode, InlineNode, InlineValue, InterpolatedText,
    InterpolatedTextPart, ListNode, Modifier, RootNode, StateVar,
};
pub use error::{Result, SakkoError};
pub use lexer::tokenize;
pub use parser::{Parser, parse_sakko};
pub use span::{LineIndex, Span};
pub use token::{Token, TokenKind};
pub use typecheck::{Diagnostic, JsEscape, Report, check_ast, check_source};

/// Reserved sentinel name used when auto-wrapping input that lacks an
/// explicit root (`<name { ... }>`).
pub const WRAPPER_NAME: &str = "__sakko_wrapper__";

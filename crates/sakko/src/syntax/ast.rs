use crate::saho::{self as x, Stmt};
use crate::span::Span;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

// Reactive snippet IR
// Every reactive payload in the document is parsed exactly once, at
// document-parse time, into a typed Saho node.

/// A single parse diagnostic captured at construction time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnippetDiag {
    pub span: Span,
    pub message: String,
}

impl From<x::lexer::ExprDiag> for SnippetDiag {
    fn from(d: x::lexer::ExprDiag) -> Self {
        Self {
            span: d.span,
            message: d.message,
        }
    }
}

impl From<&x::lexer::ExprDiag> for SnippetDiag {
    fn from(d: &x::lexer::ExprDiag) -> Self {
        Self {
            span: d.span,
            message: d.message.clone(),
        }
    }
}

/// The pre-parsed form of a single reactive expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExprSnippet<'a> {
    /// The exact source of the expression (rebuilt from the document tokens).
    pub raw: Cow<'a, str>,
    /// The parsed Saho expression, when lexing/parsing succeeded.
    pub parsed: Option<Box<x::Node>>,
    /// Any parse error(s) produced at construction; reported by the
    /// typechecker without re-parsing.
    pub errors: Vec<SnippetDiag>,
}

/// The pre-parsed form of a reactive statement block (`@effect`, `@on`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSnippet<'a> {
    pub raw: Cow<'a, str>,
    pub parsed: Option<Vec<Stmt>>,
    pub errors: Vec<SnippetDiag>,
}

impl<'a> ExprSnippet<'a> {
    /// Parse the given expression source once. Never fails: failures are
    /// captured in [`Self::errors`].
    pub fn parse(raw: Cow<'a, str>) -> Self {
        match x::parse(&raw) {
            Ok(node) => Self {
                parsed: Some(Box::new(node)),
                errors: Vec::new(),
                raw,
            },
            Err(e) => Self {
                parsed: None,
                errors: vec![e.into()],
                raw,
            },
        }
    }
}

impl<'a> BlockSnippet<'a> {
    /// Parse the given block source once. Never fails: failures are captured
    /// in [`Self::errors`].
    pub fn parse(raw: Cow<'a, str>) -> Self {
        match x::parse_body(&raw) {
            Ok(parsed) => Self {
                parsed: Some(parsed),
                errors: Vec::new(),
                raw,
            },
            Err(diags) => Self {
                parsed: None,
                errors: diags.iter().map(Into::into).collect(),
                raw,
            },
        }
    }
}

// Document AST

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Modifier<'a> {
    Flag {
        value: Cow<'a, str>,
    },
    Pair {
        key: Cow<'a, str>,
        value: Cow<'a, str>,
    },
    #[serde(rename = "atcode")]
    Atcode {
        name: Cow<'a, str>,
        body: AtcodeBody<'a>,
    },
    Event {
        event: Cow<'a, str>,
        handler: BlockSnippet<'a>,
    },
}

/// The typed payload carried by an `@atcode` directive. Every reactive or
/// otherwise code-shaped body is pre-parsed; only truly literal payloads
/// (a `@bind` signal path, `@style` CSS text) remain plain text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AtcodeBody<'a> {
    /// Literal, non-code body: `@bind signal`, `@style "css"`, `@style { ... }`,
    /// static `@class:name`.
    Text(Cow<'a, str>),
    /// A reactive expression: `@if="..."`, `@class="..."`/`@class={...}`.
    Expr(ExprSnippet<'a>),
    /// `@each item in expr`.
    Each(EachSpec<'a>),
}

/// The `@each` directive: an item name bound over a (typed) iterable source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EachSpec<'a> {
    pub item: Cow<'a, str>,
    pub source: ExprSnippet<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InterpolatedTextPart<'a> {
    Text { value: Cow<'a, str> },
    Expr { value: ExprSnippet<'a> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolatedKind {
    #[serde(rename = "interpolated")]
    Interpolated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterpolatedText<'a> {
    #[serde(rename = "type")]
    pub kind: InterpolatedKind,
    pub parts: Vec<InterpolatedTextPart<'a>>,
}

impl<'a> InterpolatedText<'a> {
    pub fn new(parts: Vec<InterpolatedTextPart<'a>>) -> Self {
        Self {
            kind: InterpolatedKind::Interpolated,
            parts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateVar<'a> {
    pub name: Cow<'a, str>,
    pub value: ExprSnippet<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedVar<'a> {
    pub name: Cow<'a, str>,
    pub expr: ExprSnippet<'a>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AtcodeDeclaration<'a> {
    State {
        declarations: Vec<StateVar<'a>>,
        line: u32,
        col: u32,
    },
    Effect {
        body: BlockSnippet<'a>,
        line: u32,
        col: u32,
    },
    Derived {
        declarations: Vec<DerivedVar<'a>>,
        line: u32,
        col: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub declarations: Vec<AtcodeDeclaration<'a>>,
    pub children: Vec<AstNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub children: Vec<AstNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub value: InlineValue<'a>,
}

/// An inline node's value: plain text or interpolated parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InlineValue<'a> {
    Plain(Cow<'a, str>),
    Interpolated(InterpolatedText<'a>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListNode<'a> {
    pub items: Vec<AstNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AstNode<'a> {
    Root(RootNode<'a>),
    Element(ElementNode<'a>),
    Inline(InlineNode<'a>),
    List(ListNode<'a>),
}

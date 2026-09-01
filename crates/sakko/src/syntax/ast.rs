use serde::{Deserialize, Serialize};
use std::borrow::Cow;

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
        body: Cow<'a, str>,
    },
    Event {
        event: Cow<'a, str>,
        handler: Cow<'a, str>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InterpolatedTextPart<'a> {
    Text { value: Cow<'a, str> },
    Expr { value: Cow<'a, str> },
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
    pub value: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedVar<'a> {
    pub name: Cow<'a, str>,
    pub expr: Cow<'a, str>,
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
        body: Cow<'a, str>,
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

//! Expression and statement AST for the Saho sub-language.

use crate::span::Span;
use serde::{Deserialize, Serialize};

/// A spanned expression node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub kind: EKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EKind {
    /// Identifier or contextual keyword used as a value.
    Ident(String),
    /// `true` / `false`.
    Bool(bool),
    Null,
    Undefined,
    This,
    Super,
    Num(String),
    Str(String),
    Template(Vec<TplPart>),
    NewTarget,
    Array(Vec<Option<Node>>),
    Object(Vec<ObjProp>),
    Fn {
        params: Vec<Pat>,
        body: Body,
    },
    Arrow {
        params: Vec<Pat>,
        body: Body,
        is_async: bool,
    },
    Call {
        callee: Box<Node>,
        args: Vec<Arg>,
        optional: bool,
    },
    New {
        callee: Box<Node>,
        args: Option<Vec<Arg>>,
    },
    Member {
        obj: Box<Node>,
        name: String,
        optional: bool,
    },
    Index {
        obj: Box<Node>,
        index: Box<Node>,
        optional: bool,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Node>,
    },
    Update {
        op: UpdateOp,
        prefix: bool,
        target: Box<Node>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Node>,
        rhs: Box<Node>,
    },
    Assign {
        op: AssignOp,
        target: Box<Node>,
        value: Box<Node>,
    },
    Cond {
        test: Box<Node>,
        cons: Box<Node>,
        alt: Box<Node>,
    },
    Seq(Vec<Node>),
    Paren(Box<Node>),
    /// `...x` in arrays, groups (arrow params), and spreads generally.
    Spread(Box<Node>),
    /// `` tag`tpl` ``
    TaggedTpl {
        tag: Box<Node>,
        tpl: Box<Node>,
    },
    /// `expr as Type`: a compile-time type assertion.
    Assert {
        expr: Box<Node>,
        ty: TypeAst,
    },
    /// `js { ... }`: raw JavaScript passthrough. The body is opaque to
    /// Sakko; only the source span is kept.
    RawJs,
}

/// A user-written type annotation (the right side of `as`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeAst {
    Number,
    Str,
    Bool,
    Null,
    Undefined,
    Unknown,
    Array(Box<TypeAst>),
    /// `T | null | undefined`
    Nullable(Box<TypeAst>),
    /// `{ name: T, ... }`: a structural object type.
    Object(Vec<ObjTyProp>),
}

/// One member of a structural object type annotation (`{ width: number }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjTyProp {
    pub name: String,
    pub ty: TypeAst,
}

impl TypeAst {
    pub fn to_ty(&self) -> crate::typecheck::Ty {
        match self {
            Self::Number => crate::typecheck::Ty::Number,
            Self::Str => crate::typecheck::Ty::Str,
            Self::Bool => crate::typecheck::Ty::Bool,
            Self::Null => crate::typecheck::Ty::Null,
            Self::Undefined => crate::typecheck::Ty::Undefined,
            Self::Unknown => crate::typecheck::Ty::Unknown,
            Self::Array(inner) => crate::typecheck::Ty::Array(Some(Box::new(inner.to_ty()))),
            Self::Nullable(inner) => {
                let base = inner.to_ty();
                crate::typecheck::Ty::union(
                    crate::typecheck::Ty::union(base, crate::typecheck::Ty::Null),
                    crate::typecheck::Ty::Undefined,
                )
            }
            Self::Object(_) => crate::typecheck::Ty::Object,
        }
    }
}

/// One part of a template literal in the AST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TplPart {
    Quasi(String),
    Expr(Node),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjProp {
    /// `{ key: value }`: key is the raw source of ident/number/string.
    Kv {
        key: Key,
        value: Node,
    },
    /// `{ shorthand }` / method shorthand is not supported yet.
    Shorthand(Node),
    Spread(Node),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Ident(String),
    Lit(String),
    Computed(Node),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Arg {
    Plain(Node),
    Spread(Node),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pat {
    Ident(String),
    Default { pat: Box<Pat>, init: Node },
    Rest(Box<Pat>),
    Array(Vec<Pat>),
    Object(Vec<ObjPatProp>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjPatProp {
    Kv { key: Key, pat: Pat },
    Shorthand(Pat),
    Rest(Pat),
}

/// Function/arrow body: either an expression or a statement block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Body {
    Expr(Box<Node>),
    Block(Vec<Stmt>),
}

/// Minimal statement set for block bodies (`@effect` bodies, function
/// bodies). Extended as corpus demands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Expr(Node),
    VarDecl {
        kw: VarKw,
        decls: Vec<(Pat, Option<Node>)>,
    },
    Return(Option<Node>),
    If {
        test: Node,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarKw {
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot,
    Typeof,
    Void,
    Delete,
    Await,
}

impl UnaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::Typeof => "typeof",
            Self::Void => "void",
            Self::Delete => "delete",
            Self::Await => "await",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateOp {
    Inc,
    Dec,
}

impl UpdateOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Inc => "++",
            Self::Dec => "--",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Nullish,
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtE,
    GtE,
    In,
    Instanceof,
    Shl,
    Shr,
    UShr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
}

impl BinOp {
    /// Precedence tiers; higher binds tighter. Mirrors JS except that `??`
    /// cannot be mixed with `&&`/`||` unparenthesized (enforced by typecheck).
    pub fn precedence(self) -> u8 {
        match self {
            Self::Nullish | Self::Or | Self::And => 1,
            Self::BitOr => 2,
            Self::BitXor => 3,
            Self::BitAnd => 4,
            Self::EqEq | Self::NotEq => 5,
            Self::Lt | Self::Gt | Self::LtE | Self::GtE | Self::In | Self::Instanceof => 6,
            Self::Shl | Self::Shr | Self::UShr => 7,
            Self::Add | Self::Sub => 8,
            Self::Mul | Self::Div | Self::Rem => 9,
            Self::Pow => 10,
        }
    }

    pub fn right_assoc(self) -> bool {
        matches!(self, Self::Pow)
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Nullish => "??",
            Self::Or => "||",
            Self::And => "&&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::BitAnd => "&",
            Self::EqEq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtE => "<=",
            Self::GtE => ">=",
            Self::In => "in",
            Self::Instanceof => "instanceof",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::UShr => ">>>",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Pow => "**",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    Nullish,
}

impl AssignOp {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Add => "+=",
            Self::Sub => "-=",
            Self::Mul => "*=",
            Self::Div => "/=",
            Self::Rem => "%=",
            Self::Pow => "**=",
            Self::Shl => "<<=",
            Self::Shr => ">>=",
            Self::UShr => ">>>=",
            Self::BitAnd => "&=",
            Self::BitOr => "|=",
            Self::BitXor => "^=",
            Self::And => "&&=",
            Self::Or => "||=",
            Self::Nullish => "??=",
        }
    }
}

/// Every assignment operator; the parser builds its matcher from this table.
pub const ASSIGN_OPS: &[AssignOp] = &[
    AssignOp::Assign,
    AssignOp::Add,
    AssignOp::Sub,
    AssignOp::Mul,
    AssignOp::Div,
    AssignOp::Rem,
    AssignOp::Pow,
    AssignOp::Shl,
    AssignOp::Shr,
    AssignOp::UShr,
    AssignOp::BitAnd,
    AssignOp::BitOr,
    AssignOp::BitXor,
    AssignOp::And,
    AssignOp::Or,
    AssignOp::Nullish,
];

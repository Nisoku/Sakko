---
type: concept
title: "API Reference"
description: "Public Rust API for the sakko crate"
source: "https://nisoku.org/S/docs/api-reference/"
path: /api-reference/
updated: 2026-10-09
okf:
  generated_by: "@docmd/plugin-okf"
  generated_at: "2026-10-09T09:00:50.306Z"
---
---
title: "API Reference"
description: "Public Rust API for the sakko crate"
order: 2
---

# Sakko API Reference

All types below are re-exported from the `sakko` crate root.

---

## Parsing

### `parse_sakko(input: &str) -> Result<RootNode>`

Parse a `.sako` source string into a document AST.

```rust
use sakko::parse_sakko;

let ast = parse_sakko(r#"<counter { text: "Hello" }>"#)?;
// ast: RootNode { name: "counter", children: [...], ... }
```

The returned `RootNode` contains:
- `name: Cow<str>` - component tag name
- `modifiers: Vec<Modifier>` - flag/pair modifiers on the root
- `declarations: Vec<AtcodeDeclaration>` - reactive block declarations (`@state`, `@derived`, `@effect`, `@if`, `@each`, `@class`, `@on`, `@bind`)
- `children: Vec<AstNode>` - child elements, inlines, and lists

---

### `tokenize(input: &str) -> Result<Vec<Token>>`

Tokenize a source string. Useful for lexing-level tooling; the parser uses
this internally.

```rust
use sakko::tokenize;

let tokens = tokenize("button(accent): Save")?;
// Token { kind: TokenKind::Ident, value: "button", ... }
```

**Token kinds:** `Lt`, `Gt`, `Lbrace`, `Rbrace`, `Lparen`, `Rparen`,
`Lbracket`, `Rbracket`, `Colon`, `Semi`, `Comma`, `Ident`, `String`,
`BacktickString`, `At`, `Equals`, `InterpStart`, `InterpEnd`, `Expr`,
`Dot`, `Plus`, `Minus`, `Star`, `Pipe`, `Ampersand`, `Bang`, `Question`,
`Percent`.

---

## Typechecking

### `check_source(src: &str) -> Result<Report>`

Parse and typecheck a `.sako` source string in one call. Returns a `Report`
containing diagnostics and raw-JS escape records.

```rust
use sakko::check_source;

let report = check_source(r#"<counter { @state { count = 0 } }>"#)?;
assert!(report.diagnostics.is_empty());
```

### `check_ast(ast: &AstNode) -> Report`

Typecheck an already-parsed AST node. `parse_sakko` returns a `RootNode`, so
wrap it as `AstNode::Root` before passing it here:

```rust
use sakko::{AstNode, check_ast, parse_sakko};

let root = parse_sakko(r#"<counter { @state { count = 0 } }>"#)?;
let report = check_ast(&AstNode::Root(root));
assert!(report.diagnostics.is_empty());
```

---

## Report types

### `Report`

```rust
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub js_escapes: Vec<JsEscape>,
}
```

### `Diagnostic`

```rust
pub struct Diagnostic {
    pub severity: Severity,   // currently always Error
    pub code: Code,           // stable code: SKT001..SKT015
    pub span: Span,
    pub snippet: String,
    pub caret: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn render(&self) -> String { /* formatted with source + caret */ }
}
```

**Diagnostic codes:**

| Code | Name | Description |
|------|------|-------------|
| SKT001 | `UnknownIdent` | Unknown identifier |
| SKT002 | `UnknownProp` | Unknown property |
| SKT003 | `NotCallable` | Value is not callable |
| SKT004 | `AssignMismatch` | Type mismatch in assignment |
| SKT005 | `BadOperand` | Invalid operand for operator |
| SKT006 | `BadUnaryOperand` | Invalid operand for unary operator |
| SKT007 | `DuplicateDecl` | Duplicate state declaration |
| SKT008 | `BadBindTarget` | `@bind` must target a state variable |
| SKT009 | `BadEachSource` | Malformed `@each` source |
| SKT010 | `RenderedFunction` | Function used where a value is rendered |
| SKT011 | `SnippetParse` | Saho parse error inside a snippet |
| SKT012 | `ConstReassign` | Cannot reassign `@derived` or constant |
| SKT013 | `UnknownUse` | Use of a value the checker cannot resolve |
| SKT014 | `ImpossibleCast` | `as` cast between disjoint concrete types |
| SKT015 | `BadClassType` | `@class` expression must yield a string or array of strings |

### `JsEscape`

One recorded `js { ... }` occurrence. Bodies are never typechecked.

```rust
pub struct JsEscape {
    pub kind_label: String,
    pub location: Option<(u32, u32)>,
    pub span: Span,
    pub body: String,
}
```

---

## AST node types

### `RootNode` / `ElementNode` / `InlineNode` / `ListNode`

```rust
pub struct RootNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub declarations: Vec<AtcodeDeclaration<'a>>,
    pub children: Vec<AstNode<'a>>,
}

pub struct ElementNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub children: Vec<AstNode<'a>>,
}

pub struct InlineNode<'a> {
    pub name: Cow<'a, str>,
    pub modifiers: Vec<Modifier<'a>>,
    pub value: InlineValue<'a>,
}

pub struct ListNode<'a> {
    pub items: Vec<AstNode<'a>>,
}

pub enum AstNode<'a> {
    Root(RootNode<'a>),
    Element(ElementNode<'a>),
    Inline(InlineNode<'a>),
    List(ListNode<'a>),
}
```

### Reactive snippets

Every reactive payload is parsed once at document-parse time into a typed
Saho node; the typechecker walks the pre-built tree (no re-parse).

```rust
pub struct ExprSnippet<'a> {
    pub raw: Cow<'a, str>,
    pub parsed: Option<Box<saho::Node>>,
    pub errors: Vec<SnippetDiag>,
}

pub struct BlockSnippet<'a> {
    pub raw: Cow<'a, str>,
    pub parsed: Option<Vec<saho::Stmt>>,
    pub errors: Vec<SnippetDiag>,
}
```

### Declaration types

```rust
pub enum AtcodeDeclaration<'a> {
    State { declarations: Vec<StateVar<'a>>, line: u32, col: u32 },
    Derived { declarations: Vec<DerivedVar<'a>>, line: u32, col: u32 },
    Effect { body: BlockSnippet<'a>, line: u32, col: u32 },
}

pub struct StateVar<'a> {
    pub name: Cow<'a, str>,
    pub value: ExprSnippet<'a>,
}

pub struct DerivedVar<'a> {
    pub name: Cow<'a, str>,
    pub expr: ExprSnippet<'a>,
}

pub struct EachSpec<'a> {
    pub item: Cow<'a, str>,
    pub source: ExprSnippet<'a>,
}
```

---

## Re-exports

From the crate root:

```rust
// Parsing
pub use syntax::ast::{AstNode, RootNode, ElementNode, InlineNode, ListNode, Modifier, ...};
pub use syntax::lexer::tokenize;
pub use syntax::parser::parse_sakko;
pub use syntax::token::{Token, TokenKind};

// Typechecking
pub use typecheck::{check_source, check_ast, Diagnostic, JsEscape, Report};

// Sahō (expression language)
pub mod saho;

// Shared types
pub use span::{Span, LineIndex};
pub use error::{Result, SakkoError};
```

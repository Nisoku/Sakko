---
title: "Sakko Documentation"
description: "The modern DSL for describing UI trees"
toc: false
---

# Sakko

Sakko is a bracket-based markup language that compiles to component trees for reactive UI. It is the front end of the S-eco stack: a typed document language, a strict expression sub-language (Sahō), and a compile-time typechecker, all Rust, all AOT.

## Quick start

```rust
use sakko::{parse_sakko, check_source};

fn main() {
    // Parse a .sako source string into a typed AST
    let ast = parse_sakko(r#"<counter { @state { count = 0 } }>"#)
        .expect("parse failed");

    // Typecheck the AST, which returns diagnostics + raw-JS escape records
    let report = check_source(r#"<counter { @state { count = 0 } }>"#)
        .expect("parse failed");

    if report.diagnostics.is_empty() {
        println!("Clean.");
    } else {
        for d in &report.diagnostics {
            eprintln!("{}", d.render());
        }
    }
}
```

### Lexing only

```rust
use sakko::tokenize;

let tokens = tokenize("button(accent): Save")?;
// Vec<Token> with line/column tracking and interpolation splits
```

## What you get

| Feature | Details |
|---------|---------|
| **Lexer** | Tokenizer with line/column tracking and `{expr}` string-interpolation splits |
| **Sahō parser** | Pratt expression parser (strict JS subset): types, arrows, `js{}`, `as` assertions |
| **Structure parser** | Recursive-descent parser for `<name { ... }>` documents |
| **AST** | Zero-copy via spans; serde on all token/AST types |
| **Typechecker** | Inference pass with stable diagnostic codes SKT001-SKT015 |
| **Reactive IR** | `@state`, `@derived`, `@effect`, `@on:event`, `@bind`, `@each`, `@if`, `@class` |

## Reactivity

Sakko compiles to Sairin signals for reactive UI:

```sako
<counter {
  @state { count = 0 }
  button @on:click { count++ }: "+"
  text: "Count: {count}"
}>
```

| Atcode | Description |
|--------|-------------|
| `@state { }` | Declare reactive state |
| `@effect { }` | Side effects that track dependencies |
| `@derived { }` | Computed values |
| `@on:event { }` | Event handlers |
| `@bind="signal"` | Two-way input binding |
| `{expr}` | Template interpolation (backtick or double-quoted strings) |
| `@if` | Conditional rendering inside handlers and effects |
| `@each` | List iteration |
| `@class="expr"` | Dynamic class string or array |

## Documentation

| Page | Description |
|------|-------------|
| [**Language Reference**](/Sakko/language-reference/) | Full Sakko syntax: blocks, modifiers, lists, reactivity |
| [**API Reference**](/Sakko/api-reference/) | Public Rust API: parse_sakko, tokenize, check_source, types |

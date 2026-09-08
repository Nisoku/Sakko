# Sakko

[![CI](https://github.com/Nisoku/Sakko/actions/workflows/ci.yml/badge.svg)](https://github.com/Nisoku/Sakko/actions/workflows/ci.yml)
[![Deploy](https://github.com/Nisoku/Sakko/actions/workflows/pages.yml/badge.svg)](https://github.com/Nisoku/Sakko/actions/workflows/pages.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

**The modern DSL (Design Sub-Language) for describing UI trees.**

Sakko is a bracket-based markup language that compiles to component trees. Write concise, readable markup. Get a structured AST, compile it to reactive JavaScript components, or use it as a standalone parser.

## What does it look like?

```sako
<card {
  heading: "Hello, world"
  text: "This compiles to an AST."
  button: "Get Started"
}>
```

That is the entire source. Sakko tokenizes it, parses it, and produces a structured AST. The AST can then be transformed into anything: Sazami web components, React VNodes, JSON, you name it.

Here is a more complex example:

```sako
<player {
  card(row center) {
    coverart(round): "album.jpg"
    details {
      text(bold): "Midnight City"
      text(dim): "M83"
    }
    controls {
      icon-btn: previous
      icon-btn(accent large): play
      icon-btn: next
    }
  }
}>
```

## Getting started

### Install

```bash
cargo add sakko
```

### Use

```rust
use sakko::{parse_sakko, tokenize};

// Tokenize source to tokens (useful for debugging)
let tokens = tokenize("button(accent): Click me")?;

// Parse to AST
let ast = parse_sakko(r#"
<card {
  heading: "Hello"
  button: "Click"
}>
"#)?;

println!("{ast:#?}");
```

The parser is zero-copy: AST nodes borrow their text straight from the source via spans, so parsing large templates allocates almost nothing beyond the token stream.

### AST Structure

The parser produces one of four node types:

| Type      | Fields                          | Description                  |
|-----------|---------------------------------|------------------------------|
| `root`    | `name`, `modifiers`, `children` | Top-level container          |
| `element` | `name`, `modifiers`, `children` | Block element with children  |
| `inline`  | `name`, `modifiers`, `value`    | Leaf element with text value |
| `list`    | `items`                         | Comma-separated group        |

Modifiers are either flags or key-value pairs:

```rust
Modifier::Flag { value: "accent".into() }
Modifier::Pair { key: "cols".into(), value: "3".into() }
```

Every token and AST node carries a `Span` (byte offsets into the source), that is resolvable to line/column via `LineIndex`.

## Syntax overview

### Root blocks

Every Sakko document has one root block wrapped in angle brackets:

```sako
<page {
  ...children
}>
```

### Block elements

Elements with children use curly braces:

```sako
card {
  heading: "Title"
  text: "Description"
}
```

### Inline elements

Elements without children use a colon:

```sako
text: Hello world
button(accent): Click me
icon: play
```

### Modifiers

Parenthesized flags or key-value pairs after the element name:

```sako
button(primary large): Submit
grid(cols 3 gap medium): [...]
card(row center curved): { ... }
input(placeholder "Email"): ""
```

### Lists

Comma-separated elements in square brackets:

```sako
row: [button: A, button: B, button: C]
```

### Comments

Single-line comments with `//`:

```sako
// This is a comment
card {
  text: Hello  // inline comment
}
```

### Reactive State

Declare reactive state with `@state`:

```sako
<counter {
  @state {
    count = 0
    step = 1
  }

  button @on:click { count++ }: "+"
  text: "Count: {count}"
}>
```

Compiles to Sairin signals. Read values with `{name}` interpolation.

### Effects

Run side effects with `@effect`:

```sako
<app {
  @state { count = 0 }

  @effect {
    console.log("Count changed:", count)
    js { document.title = `Count: ${count}` }
  }

  button @on:click { count++ }: "Increment"
}>
```

### Derived State

Compute derived values with `@derived`:

```sako
<app {
  @state { items = [] }

  @derived {
    count = items.length
    isEmpty = items.length == 0
  }

  text: "{count} items"
}>
```

### Event Handlers

Handle events with `@on:event`:

```sako
button @on:click { count++ }: "Click"
input @on:input { value = e.target.value as string }: ""
div @on:mouseenter { isHovered = true }: "Hover me"
```

The event parameter `e` has type `unknown`: navigate and call freely, but
assert with `as` before using it as a typed value.

### Raw JavaScript: `js { ... }`

Saho is deliberately small. When you need the full platform, escape with a
raw `js` block. The bytes pass through untouched, so any valid JavaScript
works:

```sako
<dash {
  @state {
    theme = js { return localStorage.getItem("theme") } ?? "dark"
    width = js { return window.innerWidth } as number
  }

  button @on:click {
    js { document.title = "Dashboard" }
  }: "Focus"
}>
```

Rules:

- A `js {}` block always has type `unknown`, wherever it appears.
- `unknown` navigates (`.prop`, `[i]`) and calls freely; results stay
  `unknown`.
- Typed consumption - arithmetic, comparisons, assigning into a typed slot -
  requires an assertion first (`as number`, `as string`, ...).
- Bodies are never checked semantically; every occurrence is recorded in the
  compile report for audits.

### Type Assertions

Cast with postfix `as`. Supported types: `number`, `string`, `boolean`,
`null`, `undefined`, `unknown`, arrays (`T[]`), and one nullable suffix
(`T | null`, `T | undefined`). Provably impossible casts are errors:

```sako
@state {
  n = raw as number          // unknown -> number: trusted
  bad = 5 as string          // error SKT014: cannot cast 'number' to 'string'
}
```

### Two-way Binding

Bind inputs with `@bind`:

```sako
<form {
  input @bind="username": ""
  input(type password) @bind="password": ""
  text: "Hello, {username}!"
}>
```

### Interpolation

Use `{expression}` in text values:

```sako
text: "Hello, {name}!"
text: "{a} + {b} = {a + b}"
text: "Items: {items.map(i => i.name).join(', ')}"
```

Expressions are parsed by **Saho** (`sakko::expr`), Sakko's embedded strict-expression language: Pratt-precedence parsing, template literals with nested substitutions, arrows, spread, optional chaining, the works. Every interpolation round-trips byte-for-byte through parse and lower.

## Compiling & Running

Sakko's target output is pre-compiled JavaScript bound to the [Sairin](https://github.com/Nisoku/Sairin) reactive runtime: components ship as plain JS plus script tags, with zero parser code in the browser.

The compiler pipeline is under active development:

| Stage                               | Status  |
|-------------------------------------|---------|
| Tokenizer + structure parser        | done    |
| Saho expression module              | done    |
| Typecheck pass (`sakko::typecheck`) | done    |
| Codegen (AST to JS)                 | planned |
| Sakumi CLI (`sakumi build`)         | planned |

Once the CLI lands, building a component will be:

```bash
sakumi build counter.sakko -o dist/
```

This emits JS you drop into a page alongside the Sairin runtime.

## Project structure

```text
crates/sakko/       Rust implementation
  src/lexer.rs      Tokenizer
  src/parser.rs     Structure parser
  src/expr/         Saho expression language (lexer + Pratt parser)
  src/ast.rs        AST types (serde-ready)
  tests/            Integration suites
Examples/           Example .sako files
Docs/               Documentation site (powered by DocMD)
```

## Development

Requires [rustup](https://rustup.rs). The pinned toolchain is selected automatically.

```bash
cargo build      # build
cargo test       # run all suites
cargo clippy     # lint (CI treats warnings as errors)
cargo fmt        # format
```

## Documentation

| Document                                                           | Summary                             |
|--------------------------------------------------------------------|-------------------------------------|
| [Language Reference](https://nisoku.org/Sakko/language-reference/) | Full Sakko syntax guide             |
| [docs.rs/sakko](https://docs.rs/sakko)                             | Rust API reference (once published) |

### Run docs locally

```bash
cd Docs
npm install
npm run dev
```

## License

[Apache License v2.0](LICENSE)

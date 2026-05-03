# Sakko

[![npm version](https://img.shields.io/npm/v/@nisoku/sakko.svg)](https://www.npmjs.com/package/@nisoku/sakko)
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
npm install @nisoku/sakko
```

### Use

```typescript
import { parseSakko, tokenize } from "@nisoku/sakko";

// Tokenize source to tokens (useful for debugging)
const tokens = tokenize('button(accent): Click me');

// Parse to AST
const ast = parseSakko(`
  <card {
    heading: "Hello"
    button: "Click"
  }>
`);

console.log(ast);
// {
//   type: 'root',
//   name: 'card',
//   children: [
//     { type: 'inline', name: 'heading', modifiers: [], value: 'Hello' },
//     { type: 'inline', name: 'button', modifiers: [], value: 'Click' }
//   ]
// }
```

### AST Structure

The parser produces one of four node types:

| Type | Fields | Description |
|---|---|---|
| `root` | `name`, `modifiers?`, `children` | Top-level container |
| `element` | `name`, `modifiers?`, `children` | Block element with children |
| `inline` | `name`, `modifiers?`, `value` | Leaf element with text value |
| `list` | `items` | Comma-separated group |

Modifiers are either flags or key-value pairs:

```typescript
{ type: 'flag', value: 'accent' }
{ type: 'pair', key: 'cols', value: '3' }
```

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
    document.title = `Count: ${count}`
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
    isEmpty = items.length === 0
  }
  
  text: "{count} items"
}>
```

### Event Handlers

Handle events with `@on:event`:

```sako
button @on:click { count++ }: "Click"
input @on:input { value = e.target.value }: ""
div @on:mouseenter { isHovered = true }: "Hover me"
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

## Compiling & Running

Sakko includes a compiler and runtime for building reactive web components.

### Compile to JavaScript

```typescript
import { parseSakko, compileComponent } from '@nisoku/sakko';

const ast = parseSakko('<counter { @state { count = 0 } }>');
const js = compileComponent(ast, { sairinImport: 'global' });
```

**Sairin Modes:**
- `'global'` (default) - Uses `window.sairin` (load via `<script>` tag)
- `'esm'` - ESM imports (use with a bundler)
- `'cjs'` - CommonJS requires (Node.js only)

### Register as Web Component

```typescript
import { parseSakko, registerSakkoComponent } from '@nisoku/sakko';

const ast = parseSakko('<my-counter { @state { count = 0 } }>');
await registerSakkoComponent(ast);

// Now <sakko-my-counter> is available
```

Requires `sairin` to be loaded globally:
```html
<script src="sairin.js"></script>
```

## Project structure

```text
Build/              Library source code
  src/
    parser/         Tokenizer and parser
    types/          TypeScript type definitions
  tests/            Tests
Examples/           Example .sako files
Docs/               Documentation (powered by DocMD)
```

## Development

```bash
cd Build
```

### Install dependencies
```bash
npm install
```

### Run tests
```bash
npm test
```

### Build
```bash
npm run build
```

## Documentation

| Document | Summary |
| --- | --- |
| [Language Reference](https://nisoku.org/Sakko/language-reference/) | Full Sakko syntax guide |
| [API Reference](https://nisoku.org/Sakko/api-reference/) | Public API surface |

### Run docs locally

```bash
cd Docs
npm install
npm run dev
```

## License

[Apache License v2.0](LICENSE)

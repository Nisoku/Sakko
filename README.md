# Sakko

**The modern DSL (Design Sub-Language) for describing UI trees.**

Sakko is a bracket-based markup language that compiles to component trees. Write concise, readable markup. Get a structured AST. Use it with Sazami, build your own renderer, or create something entirely different.

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
| [Language Reference](https://nisoku.github.io/Sakko/language-reference/) | Full Sakko syntax guide |
| [API Reference](https://nisoku.github.io/Sakko/api-reference/) | Public API surface |

### Run docs locally

```bash
cd Docs
npm install
npm run dev
```

## License

[Apache License v2.0](LICENSE)

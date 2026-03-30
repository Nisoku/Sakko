---
title: "Sakko Documentation"
description: "The modern DSL (Design Sub-Language) for describing UI trees"
toc: false
---

# Sakko

The modern DSL (Design Sub-Language) for describing UI trees. Sakko is a bracket-based markup language that compiles to component trees.

## Quick start

```typescript
import { parseSakko, tokenize } from "@nisoku/sakko";

// Tokenize Sakko source to tokens
const tokens = tokenize('button(accent): Click me');

// Parse to AST
const ast = parseSakko('<page { button(accent): Click me }>');
```

## What you get

| Feature | Details |
| --- | --- |
| **Tokenizer** | Lexical analysis with line/column tracking |
| **Parser** | Full AST generation with error messages |
| **Types** | TypeScript definitions included |
| **Zero deps** | Pure TypeScript, no external dependencies |
| **Reactivity** | @state, @effect, @derived, @on:event, @bind, {interpolation} |

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
| --- | --- |
| `@state { }` | Declare reactive state |
| `@effect { }` | Side effects that track dependencies |
| `@derived { }` | Computed values |
| `@on:event { }` | Event handlers |
| `@bind="signal"` | Two-way input binding |
| `{expr}` | Template interpolation |

## Documentation

| Page | Description |
| --- | --- |
| [**Language Reference**](/Sakko/language-reference/) | Full Sakko syntax: blocks, modifiers, lists, void elements |
| [**API Reference**](/Sakko/api-reference/) | Complete public API: parseSakko, tokenize, types |

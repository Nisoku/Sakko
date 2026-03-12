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

## Documentation

| Page | Description |
| --- | --- |
| [**Language Reference**](/Sakko/language-reference/) | Full Sakko syntax: blocks, modifiers, lists, void elements |
| [**API Reference**](/Sakko/api-reference/) | Complete public API: parseSakko, tokenize, types |

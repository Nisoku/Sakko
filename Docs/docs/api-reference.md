---
title: "API Reference"
description: "Public API for the Sakko parser and tokenizer"
order: 2
---

# Sakko API Reference

---

## Parser

### `parseSakko(input: string): RootNode`

Parse a Sakko source string into an AST.

```typescript
import { parseSakko } from '@nisoku/sakko';

const ast = parseSakko('<page { text: Hello }>');
// Returns: { type: "root", name: "page", children: [...] }
```

**Parameters:**
- `input: string` - Sakko source code

**Returns:** `RootNode`

**Throws:** `Error` with descriptive message on parse failure.

---

### `tokenize(input: string): Token[]`

Tokenize a Sakko source string into a token array.

```typescript
import { tokenize } from '@nisoku/sakko';

const tokens = tokenize('button(accent): Save');
// Returns: [
//   { type: "IDENT", value: "button" },
//   { type: "LPAREN", value: "(" },
//   { type: "IDENT", value: "accent" },
//   { type: "RPAREN", value: ")" },
//   { type: "COLON", value: ":" },
//   { type: "IDENT", value: "Save" }
// ]
```

---

## Types

### AST Node Types

```typescript
type RootNode = {
  type: 'root';
  name: string;
  modifiers?: Modifier[];
  children: ASTNode[];
};

type ElementNode = {
  type: 'element';
  name: string;
  modifiers?: Modifier[];
  children: ASTNode[];
};

type InlineNode = {
  type: 'inline';
  name: string;
  modifiers?: Modifier[];
  value: string;
};

type ListNode = {
  type: 'list';
  items: ASTNode[];
};

type ASTNode = RootNode | ElementNode | InlineNode | ListNode;
```

### Modifier Types

```typescript
type Modifier =
  | { type: 'flag'; value: string }
  | { type: 'pair'; key: string; value: string };
```

### Token Type

```typescript
type Token = {
  type: string;
  value: string;
  line: number;
  col: number;
};
```

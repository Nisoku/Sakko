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
  declarations?: AtcodeDeclaration[];
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
  value: string | InterpolatedText;
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
  | { type: 'pair'; key: string; value: string }
  | { type: 'event'; event: string; handler: string }
  | { type: 'atcode'; name: string; body: string };
```

### Atcode Declaration Types

```typescript
type AtcodeDeclaration =
  | {
      type: 'state';
      declarations: Array<{ name: string; value: string }>;
      line: number;
      col: number;
    }
  | {
      type: 'effect';
      body: string;
      line: number;
      col: number;
    }
  | {
      type: 'derived';
      declarations: Array<{ name: string; expr: string }>;
      line: number;
      col: number;
    };
```

### Interpolated Text

```typescript
type InterpolatedText = {
  type: 'interpolated';
  parts: Array<
    | { type: 'text'; value: string }
    | { type: 'expr'; value: string }
  >;
};
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

---

## Compiler

### `compileComponent(root: RootNode): string`

Compile a Sakko AST to JavaScript with Sairin signals.

```typescript
import { parseSakko, compileComponent } from '@nisoku/sakko';

const ast = parseSakko('<counter { @state { count = 0 } }>');
const js = compileComponent(ast);
// Returns: import { signal } from '@nisoku/sairin'; ...
```

---

## Runtime

### `registerSakkoComponent(ast: RootNode): void`

Register a component as a custom element.

```typescript
import { parseSakko, registerSakkoComponent } from '@nisoku/sakko';

const ast = parseSakko('<my-counter { @state { count = 0 } }>');
registerSakkoComponent(ast);
// Now <sakko-my-counter> is available as a web component
```

### `getComponent(name: string): RegisteredComponent`

Get a registered component.

```typescript
import { getComponent } from '@nisoku/sakko';

const comp = getComponent('my-counter');
console.log(comp.source); // The compiled JS source
```

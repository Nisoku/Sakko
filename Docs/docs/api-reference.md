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

**Token Types:**
- Identifiers and symbols: `IDENT`, `LT`, `GT`, `LBRACE`, `RBRACE`, `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `COLON`, `SEMI`, `COMMA`, `DOT`, `PLUS`, `MINUS`, `STAR`, `AT`, `EQUALS`
- String literals: `STRING`
- Interpolation tokens: `INTERP_START`, `INTERP_END`, `EXPR`

**Throws:** `Error` for unterminated strings or invalid syntax.

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

### `compileComponent(root: RootNode, options?: CompileOptions): string`

Compile a Sakko AST to JavaScript with Sairin signals.

```typescript
import { parseSakko, compileComponent } from '@nisoku/sakko';

const ast = parseSakko('<counter { @state { count = 0 } }>');
const js = compileComponent(ast);
// Returns: const { signal } = sairin; ... (global mode by default)
```

#### CompileOptions

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `id` | `string` | auto-generated | Custom component ID |
| `sairinImport` | `'global' \| 'esm' \| 'cjs'` | `'global'` | How to import Sairin |
| `sairinGlobal` | `string` | `'sairin'` | Global name for `global` mode |
| `sairinModule` | `string` | `'sairin'` | Module path for `esm`/`cjs` modes |

**Sairin Import Modes:**
- `'global'`: References `window.sairin` (requires `<script src="sairin.js">`)
- `'esm'`: Generates `import { signal } from 'sairin'` (use with bundler)
- `'cjs'`: Generates `const { signal } = require('sairin')` (Node.js only)

---

## Runtime

### `registerSakkoComponent(ast: RootNode, options?: RegisterOptions): Promise<void>`

Register a component as a custom element. **Async** in browser environments.

```typescript
import { parseSakko, registerSakkoComponent } from '@nisoku/sakko';

const ast = parseSakko('<my-counter { @state { count = 0 } }>');
await registerSakkoComponent(ast);
// Now <sakko-my-counter> is available as a web component
```

#### RegisterOptions

Same as `CompileOptions`: `sairinImport`, `sairinGlobal`, `sairinModule`.

**Note:** In browsers, only `'global'` mode is supported. For `'esm'` or `'cjs'`, call `compileComponent` separately and use a bundler.

### `getComponent(name: string): Readonly<RegisteredComponent> | undefined`

Get a registered component by name (case-insensitive).

```typescript
import { getComponent } from '@nisoku/sakko';

const comp = getComponent('my-counter');
if (comp) {
  console.log(comp.source); // The compiled JS source
  console.log(comp.factory); // The factory function
  console.log(comp.dispose); // Cleanup function for instances
}
```

### `getAllComponents(): ReadonlyMap<string, Readonly<RegisteredComponent>>`

Get all registered components.

### `getComponentSource(name: string): string | undefined`

Get the compiled source for a component by name.

### RegisteredComponent

```typescript
interface RegisteredComponent {
  readonly name: string;              // Component name (lowercase)
  readonly factory: (id?: string) => HTMLElement;  // Instance factory
  readonly dispose: (id: string) => void;  // Cleanup function
  readonly source: string;            // Compiled JS source
}
```

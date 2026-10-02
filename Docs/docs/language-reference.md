---
title: "Sakko Language Reference"
description: "Complete syntax guide for the Sakko DSL"
order: 1
---

# Sakko Language Reference

Sakko is a bracket-based markup language for describing UI trees. It compiles
to Sairin component trees, which render through Sazami. The expression
sub-language used inside reactive payloads is called **Sahō** (作法), which is a
strict, typechecked JavaScript subset.

---

## File extension

Sakko files use the `.sako` extension.

---

## Syntax overview

### Root blocks

Every `.sako` file has one root block wrapped in angle brackets:

```sako
<name {
  ...children
}>
```

The parser also accepts a bare `name { ... }` without angle brackets, using the
internal sentinel name `__sakko_wrapper__`. Prefer explicit `<tagname { ... }>`
syntax to avoid internal names in your artifacts.

**Example:**

```sako
<page {
  card { text: Hello }
}>
```

### Block elements

Block elements contain child elements inside curly braces:

```sako
element {
  child1: value
  child2 { ... }
}
```

### Inline elements

Inline elements have no children: just a name, optional modifiers, and a value:

```sako
name: value
name(modifiers): value
```

### Void elements

Elements that need no value or children can stand alone:

```sako
divider;
spacer(large);
```

These are parsed as inline elements with an empty value.

---

## Modifiers

Modifiers are placed in parentheses after the element name.

### Flags (boolean)

Space-separated tokens:

```sako
button(accent large bold): Save
card(curved): { ... }
text(dim small): Label
```

### Key-value pairs

Some tokens take a following value. Values can be bare identifiers or quoted
strings:

```sako
grid(cols 3 gap large): [...]
input(placeholder "Enter your name" type "email"): ""
```

The parser recognizes keys such as `cols`, `gap`, `radius`, `size`, `variant`,
`layout`, `placeholder`, `type`, `src`, `alt`, `icon`, `label`, `value`,
`min`, `max`, `step`, `name`, `heading`, `slot`, `active`, `open`, `message`,
`title`, `disabled`, `checked`, `selected`, `removable`, `required`,
`multiple`, `for`, `direction`, and responsive variants like `md:cols`.

> **Strict validation:** Unknown modifier flags are errors. Custom modifiers can
> be added to the recognized set.

---

## Lists

Lists group multiple sibling elements, with items separated by commas:

```sako
<grid(cols 3) {
  card { text: One },
  card { text: Two },
  card { text: Three }
}>
```

Lists appear after a colon or directly:

```sako
row: [button: A, button: B]
row {button: A, button: B}
```

### Inline siblings with semicolons

Use semicolons to place multiple inline elements on one line:

```sako
<controls {
  button: play; button: pause; badge(accent): LIVE
}>
```

---

## Strings

- **Bare words** for simple values (no spaces): `text: Hello`
- **Quoted strings** for values with spaces or special characters: `text: "Hello World"`
- **Interpolated strings** in double quotes support `{expr}`:

```sako
text: "Price = {price * qty}"
```

In expression snippets (`@state`, `@derived`, etc.) both backtick
(`` `Price = ${price}` ``) and double-quote (`"Price = {price}"`) forms
interpolate.

---

## Comments

Single-line comments start with `//`:

```sako
// This is a comment
<card {
  // Comments work anywhere
  text: Content // Inline comments too
}>
```

---

## Reactivity

Sakko supports inline reactivity through atcode declarations. These compile to
Sairin signals for reactive UI.

### State

Declare reactive state with `@state`:

```sako
<counter {
  @state {
    count = 0
    step = 1
  }

  text: "Count: {count}"
}>
```

`@state` declares variables that can be read and written. Values are
initialized with Sahō expressions. Types may be annotated with `as`:

```sako
@state {
  count = 0
  label = "x" as string
}
```

### Derived state

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

Derived values are computed (immutable); reassigning one is an error
(SKT010).

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

`document`/`window` are **not** in the global manifest. DOM access like
`document.title` works only inside `js { ... }` escape blocks.
Effects run when any referenced state changes.

### Event handlers

Handle DOM events with `@on:event`:

```sako
button @on:click { count++ }: "Click"
input @on:input { value = e.target.value }: ""
```

The handler body is Sahō. The event parameter `e` has type `unknown`;
navigation and calls on `unknown` are free, and you may assert its shape with
`as`.

### Two-way binding

Bind input elements with `@bind`:

```sako
<input @bind="username": "">
<checkbox @bind="checked": "">
<select @bind="value": "">
```

The bound signal syncs automatically with the input value. `@bind` must target
a `@state` variable (SKT007 otherwise).

### Conditionals (`@if`)

`if` statements are supported inside handlers and `@effect` bodies:

```sako
button @on:click {
  email = email.trim()
  if emailValid {
    submitted = true
  } else {
    error = "Invalid email"
  }
}: "Submit"
```

### Iteration (`@each`)

Iterate over an array with `@each="item in source"`:

```sako
<todo {
  @state {
    items = []
    visible = []
  }

  column(gap small) @each="item in visible": [
    row @on:click { item.done = !item.done }:
      [checkbox(checked = item.done), text: item.text]
  ]
}>
```

The loop variable is typed from the array's element type.

### Reactive classes

Add dynamic CSS classes with `@class`:

```sako
<div @class="theme">content</div>
<button @class={isActive ? "active" : "inactive"}>...</button>
```

The `@class` directive accepts:
- A **string** expression: `@class="theme"`
- An **array** of class names: `@class=["bold", "large"]`
- A **braced** expression: `@class={theme}`, `@class={count > 5 ? "warn" : ""}`

The value is typechecked as a class-string (string or array of strings,
SKT015 otherwise). Class changes are reflected immediately on the element.

> **Note:** object-form `@class={ key: cond }` is not supported and produces
> SKT015. Use a `@derived` string/array expression instead.

### Interpolation

Use `{expression}` in text values:

```sako
text: "Hello, {name}!"
text: "{a} + {b} = {a + b}"
text: "Items: {items.length}"
```

Interpolation compiles to effects that update the text when dependencies
change.

---

## Sahō expression language

The embedded expression language is a strict JS subset.

### Equality is always strict

`==` / `!=` are the only comparison operators and are **strict** (no coerced
comparison). `===` and `!==` are hard errors: "Saho equality is always strict;
use '=='".

### Unknown and `as` assertions

Values from dynamic sources (`e`, `js { }`, API data) have type `unknown`:

- Navigation (`.prop`, `[i]`) and calls on `unknown` are **free** (result
  `unknown`).
- Arithmetic, comparisons, and typed assignment require an `as` assertion
  first.
- New postfix type assertions:

```sako
x as number
items as string[]
maybe as string | null
obj as { width: number, height: number }
```

Impossible casts between disjoint concrete types are rejected (SKT013).

### `js { }` escape hatch

`js { ... }` is the Rust-`unsafe` analogy: the body is emitted byte-for-byte,
always types as `unknown`, and is recorded in the compile report
(`Report::js_escapes`) for audits. Use it for code Sahō cannot express:

```sako
@state {
  screen = js {
    if (typeof window !== "undefined") {
      return { width: window.innerWidth, height: window.innerHeight }
    }
    return { width: 0, height: 0 }
  } as { width: number, height: number }
}
```

The body is syntax-validated at build time (never semantically checked).

### Builtins

`console`, `Math`, `JSON`, `Number`, `String`, `Array` methods, plus `fetch`,
`setTimeout`, `clearTimeout`, `setInterval`, `clearInterval`. DOM globals are
reachable only through `js { }`.

---

## Complete example

```sako
<todo {
  @state {
    input = ""
    items = []
    filter = "all"
  }

  @derived {
    remaining = items.filter(item => !item.done).length
    visible = filter == "active"
      ? items.filter(item => !item.done)
      : filter == "done"
        ? items.filter(item => item.done)
        : items
  }

  heading(xlarge): "Todos"

  column(gap small): [
    row(gap small): [
      input @bind="input" placeholder="What needs doing?": "",
      button(accent) @on:click {
        input = input.trim()
        if input != "" {
          items.push({ text: input, done: false })
          input = ""
        }
      }: "Add"
    ],
    row: [
      button(filter == "all" ? accent : none) @on:click { filter = "all" }: "All",
      button(filter == "active" ? accent : none) @on:click { filter = "active" }: "Active",
      button(filter == "done" ? accent : none) @on:click { filter = "done" }: "Done"
    ],
    text(dim small): "{remaining} remaining",
    column(gap small) @each="item in visible": [
      row @on:click { item.done = !item.done }:
        [checkbox checked="item.done", text(item.done ? "strikethrough" : ""): item.text]
    ]
  ]
}>
```

---

## AST output

The parser produces an AST with node types derived from the document language:

| Node Type | Fields | Description |
|-----------|--------|-------------|
| `RootNode` | `name`, `modifiers`, `declarations`, `children` | Top-level container |
| `ElementNode` | `name`, `modifiers`, `children` | Block element with children |
| `InlineNode` | `name`, `modifiers`, `value` | Leaf element with text value |
| `ListNode` | `items` | Comma-separated group |

Reactive payloads are pre-parsed into typed `ExprSnippet` / `BlockSnippet`
nodes at parse time; the typechecker walks them directly.

Modifiers are `{ type: "flag", value }` or `{ type: "pair", key, value }`.
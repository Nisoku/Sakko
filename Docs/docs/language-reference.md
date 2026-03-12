---
title: "Sakko Language Reference"
description: "Complete syntax guide for the Sakko bracket-based DSL"
order: 1
---

# Sakko Language Reference

The Sakko DSL is a bracket-based markup language for describing UI trees. It compiles to Sazami component trees which render to the DOM.

---

## File Extension

Sakko files use the `.sako` extension.

---

## Syntax Overview

### Root Blocks

Every `.sako` file has one root block wrapped in angle brackets:

```sako
<name {
  ...children
}>
```

> **Note:** The `compileSakko()` function auto-wraps source that is missing `<>` brackets, so both `<card { ... }>` and `card { ... }` work when using the API. However, always have the habit to add `<>` brackets.

**Example:**

```sako
<page {
  card { text: Hello }
}>
```

### Block Elements

Block elements contain child elements inside curly braces:

```sako
element {
  child1: value
  child2 { ... }
}
```

**Example:**

```sako
<card {
  text(bold): "Title"
  text(dim): "Subtitle"
  button: "Click"
}>
```

### Inline Elements

Inline elements have no children: just a name, optional modifiers, and a value:

```sako
name: value
name(modifiers): value
```

**Example:**

```sako
text: Hello
button(accent large): Save
icon: play
```

### Void Elements

Elements that need no value or children can stand alone:

```sako
divider;
spacer(large);
```

These are parsed as inline elements with an empty value. Useful for separators, spacers, and structural markers.

---

## Modifiers

Modifiers are placed in parentheses after the element name. They configure the element's appearance and behavior.

### Flags (Boolean)

Space-separated tokens:

```sako
button(accent large bold): Save
card(curved): { ... }
text(dim small): Label
```

### Key-Value Pairs

Some tokens take a following value. Values can be bare identifiers or quoted strings:

```sako
grid(cols 3 gap large): [...]
row(gap medium): [...]
input(placeholder "Enter your name" type "email"): ""
image(src "photo.jpg" alt "A photo"): ""
```

The parser recognizes these keys as pairs: `cols`, `gap`, `radius`, `size`, `variant`, `layout`, `placeholder`, `type`, `src`, `alt`, `icon`, `label`, `value`, `center-point`.

### Modifier Categories

| Category | Values | Maps To |
|----------|--------|---------|
| **Variant** | `accent`, `primary`, `secondary`, `danger`, `success` | `variant` attribute |
| **Tone** | `dim`, `dimmer` | `tone` attribute |
| **Size** | `small`, `medium`, `large`, `xlarge` | `size` attribute |
| **Weight** | `bold`, `normal`, `light` | `weight` attribute |
| **Layout** | `row`, `column` | `layout` attribute |
| **Alignment** | `center`, `start`, `end` | `align` attribute |
| **Justify** | `space-between`, `space-around` | `justify` attribute |
| **Shape** | `round`, `square`, `pill` | `shape` attribute |
| **Curvomorphism** | `curved`, `flat` | `curved` attribute |
| **State** | `disabled`, `active`, `loading`, `checked` | boolean attributes |
| **Wrap** | `wrap`, `nowrap` | `wrap` attribute |

> **Strict validation:** Unknown modifier flags throw an error. If you need a custom modifier, add it to `MODIFIER_MAP` before use.

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

Lists can appear after a colon or directly:

```sako
row: {button: A, button: B}
row {button: A, button: B}
```

---

## Inline Siblings with Semicolons

Use semicolons to place multiple inline elements on one line:

```sako
<controls {
  button: play; button: pause; badge(accent): LIVE
}>
```

Semicolons work at any nesting level, including root:

```sako
<page {
  text: A; text: B; text: C
}>
```

---

## Strings

- **Bare words** for simple values (no spaces): `text: Hello`
- **Quoted strings** for values with spaces or special characters: `text: "Hello World"`

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

## Complete Example

```sako
<player {
  // Main card with horizontal layout
  card(row medium center curved) {
    coverart(round): "album.jpg"

    details(column gap small) {
      text(bold large): "Midnight City"
      text(dim): "M83"
      badge(accent): "Synthwave"
    }

    controls(row gap small) {
      icon-btn: previous;
      icon-btn(accent large): play;
      icon-btn: next
    }
  }

  // Queue section
  card(curved medium) {
    heading: "Up Next"

    column(gap small): [
      row(space-between gap medium) {
        text: "Track 1"
        text(dim small): "3:45"
      },
      row(space-between gap medium) {
        text: "Track 2"
        text(dim small): "4:12"
      }
    ]
  }

  // Bottom controls
  row(space-between gap large) {
    button(dim): Library
    button(accent): Discover
    button(dim): Settings
  }
}>
```

---

## AST Output

The parser produces an AST with these node types:

| Node Type | Fields | Description |
|-----------|--------|-------------|
| `root` | `name`, `modifiers`, `children` | Top-level container |
| `element` | `name`, `modifiers`, `children` | Block element with children |
| `inline` | `name`, `modifiers`, `value` | Leaf element with text value |
| `list` | `items` | Comma-separated group |

Modifiers are either `{ type: "flag", value: string }` or `{ type: "pair", key: string, value: string }`.

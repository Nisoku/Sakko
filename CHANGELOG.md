# Changelog

## [Unreleased]

### Crate refactor

Internal restructuring of the `sakko` crate with no public API changes:

- `sakko::expr` renamed to `sakko::saho`; the flat `{lexer,parser,ast,token}`
  modules now live under `sakko::syntax`, split into focused submodules
  (`syntax::parser::{text,elements,modifiers,atcodes}`,
  `typecheck::{checker,driver,report}`).
- Operator spellings have a single source of truth (`symbol()` methods plus
  an `ASSIGN_OPS` table driving both AST conversion and parsing).
- Hard-denied `clippy::unwrap_used` / `expect_used` / `panic`; all
  fallible lexer/parser paths now propagate errors instead. Unit tests for
  the type lattice moved to an integration test target.

### RiiR: the compiler is now Rust

The TypeScript toolchain (Build/, goldens, differential tests, Node CI) is
gone. Sakko is a pure Rust workspace:

- `sakko` - lexer, parser, AST, Saho expression language (`sakko::expr`),
  and the new typecheck pass. Zero runtime dependencies beyond serde and
  chumsky; every snippet round-trips byte-for-byte through parse/lower.
- `sazami` (planned) - code generator.
- `sakumi` (planned) - thin CLI on top.

CI builds on Linux/macOS/Windows with clippy, rustfmt, docs, MSRV 1.85,
and `cargo-deny` supply-chain checks.

### Typecheck pass

New `sakko::typecheck` module with stable diagnostic codes SKT001-SKT014:
unknown identifiers/properties, callability, assignment mismatches,
operand rules, duplicate declarations, bad bind targets, malformed `@each`,
rendered functions, snippet parse errors, const reassignment, unknown-value
consumption gating, and impossible casts. Diagnostics render with source
snippets and carets.

### Saho surface syntax

- Equality is always strict: `==` / `!=` only. `===` and `!==` are hard
  errors.
- Values from dynamic sources have type `unknown`: navigation and calls
  flow freely; arithmetic, comparisons, and typed assignment require an
  `as` assertion first.
- New postfix type assertions: `x as number`, `items as string[]`,
  `maybe as string | null`. Impossible casts are rejected.
- The event parameter `e` in handlers is `unknown`.
- New raw escape hatch `js { ... }`, usable as an expression or statement.
  Bodies pass through byte-for-byte, always type as `unknown`, and are
  recorded in the compile report (`Report::js_escapes`) for audits.
- New builtins: `fetch`, `setTimeout`, `clearTimeout`, `setInterval`,
  `clearInterval`. DOM objects remain reachable only through `js {}`.

### Typed reactive AST + typed `@class`

The reactive payloads are now built as pre-parsed, typechecked Saho nodes
instead of raw strings:

- `@state` / `@derived` run through the same typed pipeline as expressions,
  producing structured `Node` trees that the typechecker walks directly (no
  parse re-entry at check time).
- New reactive typed `@class` (`@class="expr"`, `@class=["a","b"]`,
  `@class={expr}`): the value is checked as a class-string (string /
  array-of-string → `Ty::Str` / `Ty::Array(Str)`). Object-form `@class` is
  intentionally not yet supported.
  Diagnostic `SKT015` covers malformed class expressions.
- `@if` statements: `if cond { ... } else { ... }` parses and typechecks in
  handlers and `@effect` blocks.
- `@each` item binding: `@each="item in source"` declares `item` with the
  element type of the iterated array.
- Object type assertions: `x as { width: number, height: number }` yields
  `Ty::Object`, letting `js { ... }` escape blocks describe their shape.
- Double-quoted strings support `{expr}` interpolation (e.g.
  `label = "Order {tier}"`) alongside the existing backtick form.
- The template-nesting depth guard (max 64) no longer relies on
  `thread_local!`, keeping `sakko` clean under `no_std` + `alloc`.
- New examples gallery in `Examples/` (counter, todo, form-validation,
  effects-js, expressions, dashboard, and more), guarded by a rot protector:
  every `Examples/*.sako` must parse and typecheck clean.

### WASM bindings (`sakko-wasm`)

New `crates/sakko-wasm` crate exposing the compiler to the browser:

- `version` / `tokenize_json` / `parse_json` / `check_json` return JSON over a
  typed DTO layer (tokens and the typed AST serialize directly; the typecheck
  report is mirrored in owned DTO shapes with a stable camelCase schema).
- `wasm-bindgen` + `js-sys` are `wasm32`-target-only dependencies; the
  bound functions throw JS `Error`s whose message is the serialized error DTO.
- Native unit tests run in the workspace gate; verified with `cargo check
  --target wasm32-unknown-unknown`.

## [0.1.6] - 2026-07-10

### Changed

- Updated ESLint to flat config with strict type rules
- Fixed all type violations across the codebase
- Fixed library name in vite.config.ts (was "Sazami", now "Sakko")

### Added

- Added CI workflow
- Added CI lint step
- Added typecheck script

## [0.1.5] - 2026-06-09

Initial public release.

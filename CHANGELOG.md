# Changelog

## [Unreleased]

### Fixed

- Panic when lexing a string literal that ends in a trailing backslash. The
  scan index ran past the end of the input, so the content slice was out of
  bounds; an unterminated-literal diagnostic is reported instead.
- Expression snippets no longer truncate interpolated strings. Reassembly
  stopped at the first `}`, so `"x {a} y {b} z"` became `"x {a}`.
- `@each` accepts any iterable source expression. Member access and calls
  (`@each="i in o.list"`, `items.filter(...)`) were treated as a bare name
  and reported as an unknown identifier.
- `if` branches no longer leak declarations. Each branch is checked in its
  own scope, so names declared inside are not visible after the `if` or in
  the other branch.
- `@class` rejects arrays whose element type is known to be non-string
  (`@class={counts}` with `counts = [1, 2]`); arrays of strings and arrays
  with an unknown element type stay valid.
- `@each="row item in xs"` is rejected; the quoted binding form now requires a
  single identifier, like the token form.
- Stack overflow on deeply nested template literals. The nesting cap was only
  enforced while parsing, after the lexer had already recursed; the limit is
  now checked during lexing, so pathological input returns the
  "template literal nested too deeply" diagnostic instead of aborting.
- `@derived` no longer silently swallows a malformed leading declaration. A
  bare identifier or missing `=` is now reported as "Expected variable
  declaration", matching `@state`.
- Method call return types now reach their call sites. The signature table
  was keyed on the receiver's span while lookups used the whole `obj.name`
  span, so every method call degraded to `any` and bogus members on the result
  went unreported.
- `@each` accepts a union of arrays and unions of strings, keeping the
  branches' common element type for the bound item. Non-iterable union
  members are still rejected.
- Windows CI: typecheck snapshots no longer mismatch under a CRLF checkout.
  Added a `.gitattributes` normalizing the working tree to LF, and the
  snapshot comparison itself now ignores CRLF differences.
- `@style` inside a parenthesized modifier list accepts the optional `=`, so
  `button(@style="color: red")` parses like `button @style="color: red"`.
- An unterminated substitution no longer hangs the parser. A `key="value"` pair
  inside parentheses was classified as a class expression, which refuses to
  start at a lone `=` and so consumed nothing. Known keys (`placeholder`,
  `data-*`) now pair up, unknown keys report the existing diagnostic, and bare
  flags such as `div(gap small)` are unaffected. A lone `=` versus `==` is
  what now distinguishes the two cases.
- `sakko-wasm`: `tokenize` and `check` now return document errors as a
  serialized `ErrorDto` (message, line, column, suggestion, snippet) instead
  of an opaque internal error type.
- `sakko-wasm`: corrected the local `sakko` path dependency requirement from
  `0.1.0` to `0.1.6` to match the workspace version.
- `async` arrow nodes span the whole expression. The span only had its end
  extended, so `g(async x => x)` reported an arrow covering `x => x` and
  dropped the `async` keyword.
- Expression snippet depth counters no longer go negative. `saturating_sub`
  floors at `i32::MIN` rather than `0`, so an unmatched closer left a depth of
  `-1`, the depth-0 stop guard never held again, and the scan ran to end of
  input instead of stopping at the next delimiter.
- Namespace methods report their real return types instead of `Function`:
  `Math` and `Number.parseFloat`/`parseInt` return `number`, `JSON.parse`
  returns `unknown`, `JSON.stringify`, `String.fromCharCode`/`fromCodePoint`/
  `raw` return `string`, `Number.isNaN`/`isFinite` return `boolean`, and
  `console` methods return `undefined`.
- The item bound by `@each` no longer picks up `null`/`undefined` from a union
  source. `flag ? items : null` bound `number | null` even though those members
  carry no element.
- A `{` inside a double-quoted JS string is only treated as the start of a
  substitution when its matching `}` closes within the same literal. `"{"` and
  `"a { b"` are valid JS and previously failed with "unterminated string
  literal"; a trailing `{` such as `"a{b}c{"` is now kept as literal text.

### Documentation

- `api-reference.md`: corrected `AtcodeDeclaration` to its struct variants
  (`State { declarations, line, col }`, `Derived { .. }`, `Effect { .. }`).
  Removed the non-existent `ty: Option<TypeAst>` field from `StateVar` and
  `DerivedVar`. `check_ast` now shows wrapping the parsed `RootNode` in
  `AstNode::Root`.
- `language-reference.md`: corrected three diagnostic codes to the ones the
  checker actually emits — derived reassignment is `SKT012` (not `SKT010`),
  an invalid `@bind` target is `SKT008` (not `SKT007`), and an impossible
  cast is `SKT014` (not `SKT013`).

### Crate refactor

Internal restructuring of the `sakko` crate with no public API changes:

- `sakko::expr` renamed to `sakko::saho`; the flat `{lexer,parser,ast,token}`
  modules now live under `sakko::syntax`, split into focused submodules
  (`syntax::parser::{text,elements,modifiers,atcodes}`,
  `typecheck::{checker,driver,report}`).
- Operator spellings have a single source of truth (`symbol()` methods plus
  an `ASSIGN_OPS` table driving both AST conversion and parsing). The
  typechecker now calls those `symbol()` methods instead of keeping a second
  copy of the table.
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
- `[lib] crate-type = ["cdylib", "rlib"]`; a release `.wasm` builds under
  `wasm32-unknown-unknown`.
- Native unit tests run in the workspace gate; an opt-in end-to-end Node check
  (`Tests/sakko-wasm/run-node-check.sh`) exercises the compiled `.wasm`
  through the generated wasm-bindgen glue.

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

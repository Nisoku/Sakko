# Changelog

## [Unreleased]

## [0.1.6] - 2026-10-08

Sakko is rewritten from scratch as a Rust workspace. The TypeScript
implementation is gone: `Build/`, the Vite/Jest setup, the old goldens and
differential tests, and the npm release workflow. The compiler, its test
suites, and the browser bindings are all Rust now.

### Added

- **`sakko` crate** - lexer, structure parser, AST, and the Saho expression
  language (chumsky-based Pratt parser). Snippets round-trip byte-for-byte
  through parse/lower.
- **Typecheck pass** (`sakko::typecheck`) with stable diagnostic codes
  SKT001-SKT015: unknown identifiers/properties, callability, assignment
  mismatches, operand rules, duplicate declarations, invalid `@bind` targets,
  malformed `@each`, snippet parse errors, const reassignment, unknown-value
  gating, and impossible casts. Diagnostics render with source snippets and
  carets.
- **Saho** - equality is always strict (`===`/`!==` rejected); values from
  dynamic sources are `unknown` and need an `as` assertion before typed use;
  postfix type assertions (`x as number`, `items as string[]`,
  `x as { width: number }`); a raw `js { ... }` escape hatch that always types
  as `unknown` and is recorded in `Report::js_escapes`; `fetch` and timer
  builtins.
- **Typed reactive AST** - `@state`/`@derived`/`@effect` compile through the
  same typed pipeline, so the checker walks the nodes directly. Typed `@class`,
  `@if`, `@each` item binding, `{expr}` interpolation in double-quoted strings,
  and `@style`.
- **`sakko-wasm`** - `version`/`tokenize_json`/`parse_json`/`check_json` over a
  typed DTO layer, `wasm-bindgen` glue whose thrown errors are serialized error
  DTOs, and an end-to-end Node smoke test.
- **Examples + tests** - an `Examples/*.sako` gallery guarded by a rot
  protector, and integration suites for tokenizing, parsing, Saho, typechecking,
  and diagnostics.
- **CI** - Linux/macOS/Windows (fmt, clippy, tests, docs, `cargo-deny`,
  MSRV 1.88), a separate WASM workflow, dependabot, and repo boilerplate
  (CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, issue/PR templates).

### Removed

- The TypeScript compiler and its toolchain (`Build/`, Vite, Jest, ESLint, and
  the npm release workflow).

## [0.1.5] - 2026-06-09

Initial public release.

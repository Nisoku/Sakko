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

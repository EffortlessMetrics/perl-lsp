# ADR-2026-0417-001: Public Query Helpers for tree-sitter-perl-c

## Status
Accepted

## Context

The `tree-sitter-perl-c` crate wraps a vendored C tree-sitter grammar and exposes a
`language()` function that returns `tree_sitter::Language`. The crate's four tree-sitter
query files (`injections.scm`, `highlights.scm`, `folds.scm`, `matchup.scm`) live at the
workspace root (`tree-sitter-perl/queries/`) but are only accessible internally via
repeated `include_str!("../../../tree-sitter-perl/queries/...")` boilerplate:

- `src/lib.rs:163` — private `const INJECTIONS_QUERY` inside `#[cfg(test)]`
- `tests/bdd_workflows.rs:126` — duplicate `include_str!` for `injections.scm`
- `tests/bdd_workflows.rs:168` — duplicate `include_str!` for `injections.scm`

The crate's own `ROADMAP.md:27-28` explicitly acknowledges this as a known limitation:
> "The crate does not expose tree-sitter query helpers — use the `tree-sitter` crate
> directly with the `language()` return value."

Callers who want injection/highlight/fold/tag queries must either duplicate the `include_str!`
pattern or rely on private, internal constants. This is a friction point for the crate's
documented role as "the conventional tree-sitter Perl grammar binding (C FFI), maintained
for compatibility and comparison against the native v3 parser."

## Decision

Implement **Option 1** from the issue: create a dedicated `src/queries.rs` submodule
that exposes a public API for all four tree-sitter query files.

### API Design

The `queries` module exports eight functions (four pairs):

| Function | Returns | Purpose |
|----------|---------|---------|
| `injections_query_str()` | `&'static str` | Raw `injections.scm` string for callers who manage `Query` construction themselves |
| `highlights_query_str()` | `&'static str` | Raw `highlights.scm` string |
| `folds_query_str()` | `&'static str` | Raw `folds.scm` string |
| `matchup_query_str()` | `&'static str` | Raw `matchup.scm` string |
| `load_injections_query()` | `Result<Query, QueryError>` | Convenience constructor: `Query::new(&language(), injections_query_str()?)?` |
| `load_highlights_query()` | `Result<Query, QueryError>` | Same pattern for `highlights.scm` |
| `load_folds_query()` | `Result<Query, QueryError>` | Same pattern for `folds.scm` |
| `load_matchup_query()` | `Result<Query, QueryError>` | Same pattern for `matchup.scm` |

Additionally:
- `pub use tree_sitter::QueryError;` is re-exported from the `queries` module so callers
  can handle errors without adding `tree-sitter` as a direct dependency
- All eight functions have doc comments describing what each query file is for

### Module Structure

```
crates/tree-sitter-perl-c/
  src/
    lib.rs         — add pub mod queries; re-export QueryError from queries
    queries.rs     — NEW: all eight query functions
  tests/
    queries.rs     — NEW: integration tests for all eight functions
```

### Path Resolution

The `include_str!` path `../../../tree-sitter-perl/queries/<file>.scm` resolves from
`src/queries.rs` to the workspace root, identical to the already-verified path at
`src/lib.rs:163`. The path is compile-time verified.

### Backward Compatibility

All existing public API (`language()`, `try_create_parser()`, `create_parser()`,
`parse_perl_code()`, `parse_perl_file()`, `get_scanner_config()`) is untouched.
The change is purely additive (semver-safe).

### ROADMAP Update

`ROADMAP.md` lines 27-28 (the "known limitation" bullet) are removed or rephrased
to reflect that the limitation has been addressed. This is a required part of the
implementation, not optional cleanup.

## Consequences

### Benefits
1. **Eliminates duplication** — three copies of the `include_str!` path collapse to one canonical source
2. **Documents the API** — callers have a public, versioned API instead of hacking private constants
3. **Matches ROADMAP commitment** — implements the explicitly documented next step from `ROADMAP.md`
4. **Follows ecosystem conventions** — mirrors the pattern used by `tree-sitter-json`, `tree-sitter-rust`, and other tree-sitter language crates
5. **Forward-looking** — `highlights.scm`, `folds.scm`, and `matchup.scm` are untested in this crate but are part of the upstream query suite; exposing them enables future tooling

### Tradeoffs / Risks
1. **Compile-time path coupling** — the relative path `../../../tree-sitter-perl/queries/` is baked in; if the workspace layout changes, this path breaks (low likelihood; path is verified at compile time)
2. **Untested query files** — only `injections.scm` is exercised by existing tests; the other three are exposed based on upstream `tree-sitter-perl` maintenance (acceptable risk; upstream is the source of truth)
3. **Runtime query errors** — `load_*_query()` functions return `Result<Query, QueryError>` so invalid `.scm` syntax surfaces at runtime (same failure mode as the existing `include_str!` + `Query::new()` pattern)
4. **API surface increase** — eight new public functions added (acceptable; all are purely additive)

## Alternatives Considered

### Alternative 1: Keep status quo (do nothing)
- **Rejected because**: The `ROADMAP.md` explicitly calls this out as a known limitation. The duplication is a maintenance hazard. Callers must duplicate `include_str!` to access query files, which is poor ergonomics for a crate that is explicitly maintained as a "compatibility baseline."

### Alternative 2: Scatter functions directly in `lib.rs`
- **Rejected because**: The issue explicitly recommends Option 1 (dedicated submodule). Scattering functions in `lib.rs` mixes query-related code with parser/grammar concerns and makes the public API surface harder to discover.

### Alternative 3: Feature-gated module (only expose `injections`, gate the others)
- **Rejected because**: The query files are already compiled into the crate via `include_str!` — there is no additional build cost to exposing all four. A feature flag would fragment the public API unnecessarily and add complexity for callers who want to use multiple query types.

### Alternative 4: Put tests inside `src/queries.rs` as `#[cfg(test)]` rather than `tests/queries.rs` integration tests
- **Rejected because**: The crate already has a pattern of integration tests in `tests/bdd_workflows.rs`. Using a separate `tests/queries.rs` integration test file is more idiomatic for this crate and keeps unit tests (`#[cfg(test)]` within `src/`) and integration tests cleanly separated.

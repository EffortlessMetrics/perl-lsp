# Perl parser coverage risk map (2026-05-05)

## Scope

This forensic note scopes parser coverage work to the parser subsystem only:

- `perl-parser` (facade and parser-facing APIs)
- `perl-parser-core` (parse engine and syntax handlers)

It intentionally does **not** raise global workspace coverage thresholds.

## Coverage collection command

Use `just coverage-parser`.

Command executed by the recipe:

```bash
rustup run nightly cargo llvm-cov \
  -p perl-parser \
  -p perl-parser-core \
  --all-features \
  --locked \
  --branch \
  --lcov \
  --output-path target/coverage/parser.lcov
```

If `cargo-llvm-cov` is not installed, the recipe exits successfully and prints installation guidance so CI/dev flows remain safe.

## Risk classification for uncovered or weakly covered areas

### 1) Critical parser behavior (highest value)

Target these first when increasing branch coverage:

- Statement parsing and control-flow branch handling
  - `crates/perl-parser-core/src/engine/parser/statements.rs`
- Syntax-specific handlers with known failure clusters
  - `crates/perl-parser-core/src/syntax/heredoc.rs`

Why critical: misses here directly change AST shape, symbol edges, and downstream editor trust.

### 2) Recovery and error handling (highest value)

Prioritize paths that determine salvage quality after malformed input:

- Recovery resynchronization decisions
- Error node containment and spillover behavior
- Post-error statement and symbol salvage

Why critical: these paths govern live-edit resilience and prevent cascading parse degradation.

### 3) Span/position correctness (high value)

Prioritize branch coverage in byte/line/UTF-16 mapping paths:

- Span construction and conversion helpers
- Off-by-one and newline-boundary branches
- Multibyte/UTF-16 offset handling

Why high value: these branches determine goto/highlight/diagnostic correctness even when parse succeeds.

### 4) Incremental parsing (high value)

Prioritize invalidation and equivalence branches:

- Changed-region invalidation decisions
- Fast-path equivalence checks
- Fallback-to-full-parse branch paths

Why high value: branch misses here can produce stale or wrong editor facts under normal typing.

### 5) Facade and re-export glue (lower value)

Examples include thin pass-through APIs, `pub use` surfaces, and module wiring in `perl-parser`.

Why lower value: these lines can be under-covered without materially reducing parser correctness, as long as core behavior branches are covered.

### 6) Deprecated compatibility surface (lower value)

Compatibility aliases and migration shims should remain tested for smoke behavior but are not a first target for branch improvements.

Why lower value: correctness risk is typically bounded and isolated compared to parser-core logic.

### 7) Generated/static data (exclude or de-prioritize)

Generated tables/static declarations are low signal for branch-quality investment unless they contain nontrivial control flow.

Why excluded/de-prioritized: increasing coverage here often does not improve parser trust or live-edit outcomes.

## Baseline policy

Parser-specific baseline lives at:

- `.ci/coverage/parser-baseline.json`

Initial floor policy:

- line coverage floor: `0.58`
- branch coverage floor: `0.40`
- critical file floors:
  - `statements.rs`: `0.70` branch
  - `heredoc.rs`: `0.75` branch

## Intended outcome

This map lets operators distinguish:

- low-value uncovered facade/compat areas, from
- high-value uncovered parser-core recovery and syntax logic.

That enables future parser PRs to target coverage work where it most improves parser trust.

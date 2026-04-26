# Parser Ratchet scope selection

This document defines **selection-only policy** for Parser Ratchet.

- Parser Ratchet should **always emit a receipt**.
- Parser Ratchet should run expensive parser work **only when parser behavior is plausibly affected**.
- This change does **not** implement parser corpus execution, comparator logic, or CPAN behavior.

## Source of truth

Selection rules live in:

- `.ci/scope.d/parser-ratchet.toml`

## Selection criteria

Parser Ratchet is selected when **any** configured parser-relevant path glob matches a changed file, or when any parser-relevant risk tag is present.

Primary path groups:

- Parser/lexer/token and parser-core crates.
- Tree-sitter parser crates.
- `xtask` parser/corpus/ratchet task wiring and `ci_scope`/`gates` changes.
- CI policy and workflow folders that can alter parser gate behavior.
- Parser/corpus test folders.
- `Cargo.lock` policy marker for parser-relevant dependency movement.

Risk tags:

- `parser`
- `lexer`
- `token`
- `corpus`
- `incremental`
- `tree-sitter`
- `parser-recovery`
- `parser-accuracy`

## Receipt behavior

- On selection: `selected: true` with `selection_reason`.
- On no-op: `selected: false` with `reason`.
- Receipt emission remains required in both cases.

## Fixtures

Parser-ratchet scope fixtures are stored under:

- `xtask/tests/fixtures/ci-scope/parser-ratchet/`

Included scenarios:

- docs-only fixture → selected false
- parser crate change → selected true
- lexer/token change → selected true
- `ci_scope.rs` change → selected true
- workflow change → selected true

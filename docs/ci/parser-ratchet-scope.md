# Parser Ratchet Scope Selection

This document describes **scope-selection only** for Parser Ratchet.

Parser Ratchet should always produce a receipt (`always_report = true`), but the
expensive parser work should only run when parser behavior is plausibly affected.

## Policy file

Scope policy lives in:

- [`.ci/scope.d/parser-ratchet.toml`](../../.ci/scope.d/parser-ratchet.toml)

## Selection behavior

A decision should always be emitted with one of these shapes:

- Selected:
  - `selected: true`
  - `selection_reason: <why selected>`
- Not selected:
  - `selected: false`
  - `reason: <why not selected>`

## Selector inputs

Parser Ratchet is selected when either condition matches:

1. A changed path matches parser-relevant globs (parser, lexer, token,
   tree-sitter, parser corpus, CI scope/gates/ratchet, parser-ratchet policy,
   workflows, parser tests, or `Cargo.lock` with parser-relevant dep movement).
2. Risk tags include parser-relevant tags:
   - `parser`
   - `lexer`
   - `token`
   - `corpus`
   - `incremental`
   - `tree-sitter`
   - `parser-recovery`
   - `parser-accuracy`

## Explicit non-parser examples

These should no-op unless they also hit one of the parser policy triggers above:

- docs-only non-parser docs
- VS Code extension-only changes
- DAP-only changes
- editor docs
- forensics docs

## Fixtures

Fixtures for scope-selection scenarios are under:

- `xtask/tests/fixtures/ci-scope/parser-ratchet/`

Included scenarios:

- docs-only fixture -> `selected: false`
- parser crate change -> `selected: true`
- lexer/token change -> `selected: true`
- `ci_scope.rs` change -> `selected: true`
- workflow change -> `selected: true`

## Out of scope in this change

This change does **not**:

- path-filter the workflow
- run parser corpus
- implement comparator behavior
- add CPAN behavior

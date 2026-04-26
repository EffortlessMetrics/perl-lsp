# Parser Ratchet scope selection

Parser Ratchet is **always reported**, but parser-expensive work should run only when
parser behavior is plausibly affected.

This policy is configured in:

- `.ci/scope.d/parser-ratchet.toml`

## Selection inputs

Parser Ratchet is selected when **any** parser-relevant signal matches:

1. Changed file path matches a parser-relevant glob.
2. Scope/risk classification includes any parser-relevant risk tag.

### Parser-relevant path families

- `crates/perl-parser/**`
- `crates/perl-parser-core/**`
- `crates/perl-lexer/**`
- `crates/perl-token/**`
- `crates/tree-sitter-perl-rs/**`
- `crates/tree-sitter-perl-c/**`
- `xtask/src/tasks/*parser*`
- `xtask/src/tasks/*corpus*`
- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `xtask/src/tasks/ratchet*.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.ci/parser-ratchet/**`
- `.github/workflows/**`
- `tests/parser/**`
- `tests/perl-corpus/**`
- `Cargo.lock` (when parser-relevant dependency movement is present)

### Parser-relevant risk tags

- `parser`
- `lexer`
- `token`
- `corpus`
- `incremental`
- `tree-sitter`
- `parser-recovery`
- `parser-accuracy`

## Receipt contract

Selection is reported in receipt-style output with explicit reason fields:

- selected case: `selected=true` with `selection_reason`
- non-selected case: `selected=false` with `reason`

This PR intentionally adds **scope-selection only**:

- no workflow-level path filtering
- no parser corpus execution wiring yet
- no comparator implementation yet
- no CPAN additions

## Fixtures

Scope fixtures for policy examples and regression coverage are in:

- `xtask/tests/fixtures/ci-scope/parser-ratchet/`

Included examples:

- docs-only fixture -> `selected:false`
- parser crate change -> `selected:true`
- lexer/token change -> `selected:true`
- `ci_scope.rs` change -> `selected:true`
- workflow change -> `selected:true`

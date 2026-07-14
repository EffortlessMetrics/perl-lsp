# CLAUDE.md (perl-dead-code)

## Role

Dead-code detection for Perl codebases: unused subroutines/variables/
constants/packages/imports/exports and unreachable/dead-branch code. The
crate's own doc comment marks this a **stub implementation** demonstrating
the architecture, not a complete analysis engine.

## Owns

- `DeadCodeType` -- the categories detected (`UnusedSubroutine`,
  `UnusedVariable`, `UnusedConstant`, `UnusedPackage`, `UnreachableCode`,
  `DeadBranch`, `UnusedImport`, `UnusedExport`).
- `DeadCode` -- a single finding (type, name, file path, line range,
  human-readable reason, confidence 0.0-1.0, optional fix suggestion).
- `DeadCodeAnalysis` / `DeadCodeStats` -- aggregate result and summary
  counters for a workspace analysis run.
- `DeadCodeDetector` -- `new(WorkspaceIndex)`, `add_entry_point(PathBuf)`,
  `analyze_file(&Path) -> Result<Vec<DeadCode>, String>`.
- `dead_branches` (private module) -- line/indentation-based unreachable
  code and dead-branch detection feeding `analyze_file`.
- `report::generate_report(&DeadCodeAnalysis) -> String` -- human-readable
  report rendering.

## Does not own

- Workspace indexing itself -- consumes a pre-built `perl-workspace`
  `WorkspaceIndex` rather than building one.
- The newer LSP-free facts substrate -- this crate depends on the legacy
  `perl-workspace` index, not `perl-workspace-core`; a migration would be a
  separate, deliberate change, not an incidental one.
- Any LSP provider wiring -- produces `DeadCode`/`DeadCodeAnalysis` values
  for a caller to present, doesn't implement a code-action or diagnostic
  provider itself.

## Neighbors

- Upstream: `perl-workspace` (for `WorkspaceIndex`, `SymbolKind`,
  URI/path helpers), `serde`.
- Downstream: none in-workspace currently.

## Read first

- `src/lib.rs` -- top doc comment ("stub implementation... to demonstrate
  the architecture"), `DeadCodeDetector`, and `analyze_file`'s line-scanning
  approach.
- `src/dead_branches.rs` -- the unreachable-code/dead-branch detection
  logic `analyze_file` delegates to.

## Focused validation

`cargo test -p perl-dead-code`. `tests/dead_code_behavior_tests.rs` covers
detection scenarios; `tests/branch_coverage_tests.rs` and
`tests/robustness_tests.rs` cover dead-branch detection and malformed-input
handling respectively.

## Review hotspots

`analyze_file`'s detection is line/indentation-based, not AST-based --
changes to detection heuristics should be checked against
`tests/error_path_coverage.rs` and `tests/coverage_gap_tests.rs` for known
false-positive/false-negative patterns before broadening any detection
rule (a false-positive "unused" finding risks the caller deleting live
code).

## Claim boundary

Describes the current stub-level detection surface as authored. Does not
assert completeness of dead-code detection across dynamic Perl idioms
(string-eval, symbol-table manipulation, AUTOLOAD) -- the crate's own doc
comment flags this as a demonstration of the architecture, not a finished
analyzer.

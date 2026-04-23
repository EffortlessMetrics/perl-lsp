# Context: Remove LSP Provider Re-exports from perl-parser (#4414)

## Problem Statement

`crates/perl-parser/Cargo.toml` has 8 LSP provider crates as dependencies purely to re-export their types for convenience (`use perl_parser::{code_actions, completion, ...}`). This creates a **dependency inversion**:

- Parser is a **leaf crate** (core language processing, no LSP)
- LSP providers are **application crates** (compose parser output into LSP features)
- Current state: parser depends on application crates (backwards)
- Correct state: application crates depend on parser (forwards)

This is the root cause preventing microcrate collapse (#4410). Parser cannot be a pure leaf with LSP-shaped dependencies hanging off it.

## Root Cause Analysis

### Why the re-exports exist

The re-exports were added to provide a single import path. Instead of:
```rust
use perl_lsp_code_actions::CodeActionsProvider;
use perl_lsp_completion::CompletionProvider;
use perl_lsp_diagnostics::DiagnosticProvider;
```

Consumers could write:
```rust
use perl_parser::{code_actions, completion, diagnostics};
```

This convenience came at the cost of architectural inversion.

### Why this matters

1. **Microcrate collapse roadmap** (#4410) requires parser to be a pure leaf crate
2. **Dependency graph correctness** — application (perl-lsp) should not be imported by foundations (perl-parser)
3. **Timing decision** — ADR-0041 (PR #4413) specifies v0.13.0 as clean-break release, not v0.13.0
4. **Scope clarity** — parser + analysis layers stay focused on language semantics; LSP features become explicit composition in `perl-lsp`

## Oppositional Planning Objection and Resolution

**Objection raised**: Why not use a feature flag (`cfg(feature = "lsp-compat")`) to gate the re-exports and maintain backward compatibility?

**Decision**: Rejected. ADR-0041 specifies a v0.13.0 clean break, not an incremental migration. Feature-gated re-exports would:
- Complicate the parser crate's public API surface (testing two configs)
- Defer the architectural fix, making follow-up work harder
- Create confusion about "the right" import path

**Trade-off accepted**: Consumer code must update imports once (documented, mechanical refactor). The long-term architectural correctness is worth the one-time cost.

## Scope Boundaries

### In Scope: Cargo-Graph Inversion (This PR)

Remove the 8 LSP provider crates from `perl-parser`'s dependencies:
- `perl-lsp-code-actions`
- `perl-lsp-completion`
- `perl-lsp-diagnostics`
- `perl-lsp-inlay-hints`
- `perl-lsp-navigation`
- `perl-lsp-rename`
- `perl-lsp-semantic-tokens`
- `perl-lsp-tooling`

All 8 have 1-1 re-export mapping in `perl-parser/src/lib.rs` (lines 437-519). Removing them is mechanical.

### Out of Scope: Source-Level Coupling (Follow-up Issue)

`crates/perl-parser/src/ide/lsp_compat/` (5084 LOC) contains full re-implementations of LSP features:
- `CodeActionProvider`, `CompletionProvider`, `DiagnosticProvider`, etc.

This is **source-level coupling**, not Cargo-graph coupling. It represents a separate architectural question:
- Are these implementations still used or active?
- Should they be consolidated with the perl-lsp providers?
- Can they be removed?

This requires deeper analysis and is deferred to a follow-up issue. The `ide/lsp_compat/` module is not deleted in this PR.

## Cascading Changes Required

Removing re-exports requires 3 types of updates:

### 1. Live Code Consumers (Compile-time)

These must be updated for the build to succeed:

- `crates/perl-lsp/src/lib.rs` (prelude): imports `perl_critic` for re-export
- `crates/perl-lsp/src/features/diagnostics/pull.rs`: imports `BuiltInAnalyzer` from perl_parser

**Strategy**: Change `perl_parser::*` imports to `perl_lsp_tooling::*` (the actual source crate).

- `crates/perl-parser/tests/ast_snapshot_tests.rs`: imports `semantic_tokens` module

**Strategy**: Import directly from `perl_lsp_semantic_tokens` with module alias to avoid changing call sites.

### 2. Documentation Examples (Non-compile-time)

These are illustrative code in doc files and doc comments. Not compiled, but should remain accurate:

- `docs/reference/LSP_IMPLEMENTATION_GUIDE.md` (3 examples)
- `docs/reference/LSP_PROVIDERS_REFERENCE.md` (3 examples)
- `docs/how-to/IMPORT_OPTIMIZER_GUIDE.md` (1 example)
- `crates/perl-lsp/src/features/implementation_provider.rs` (1 doc comment)

**Strategy**: Update examples to show the correct direct imports from provider crates.

### 3. Refactor Tracking

Two legitimate re-exports are **preserved** because they don't create inversion:
- `refactor::*` (from perl-refactoring, which is also a leaf crate)
- `tokens::*` (from perl-parser-core, an internal crate)

These stay in `lib.rs` (lines 498-513).

## Risk Assessment

**Risk level**: Low

Justification:
1. **Pure removal** — no new features, no behavior changes
2. **All call sites identified** — 10 update locations (2 live code, 8 doc)
3. **No external API breakage for consumers of perl-parser** — this crate is primarily used internally by perl-lsp
4. **Test coverage exists** — `ast_snapshot_tests.rs` validates semantic_tokens import works

**Compiler guarantee**: Rust's type system will catch any missed imports at compile time.

## Related Documents

- **#4410**: Microcrate collapse roadmap — tracks the full sequence of dependency unwinding
- **ADR-0041** (PR #4413): Architectural Decision Record — v0.13.0 clean break, rationale for timing, alternatives considered
- **CLAUDE.md** (perl-lsp): Crate structure diagram showing current dependencies and planned state

## Precedent in Codebase

Similar refactors:
- **#3866**: Extracted perl-lsp-completion into its own crate (inverse of this change)
- **#3952**: Removed inline LSP provider code from perl-parser main.rs

These established the pattern of moving features from core to leaf crates.

## Verification Strategy

After implementation:

1. **Dependency graph check**: `cargo tree -p perl-parser --edges normal | grep perl-lsp-` → must be empty
2. **Build verification**: `cargo build -p perl-parser --release` and `cargo build -p perl-lsp-rs --release` → both green
3. **Test validation**: `cargo test -p perl-parser` and `cargo test -p perl-lsp-rs` → all pass
4. **Lint check**: `cargo clippy -p perl-parser` → no warnings
5. **Format check**: `cargo xtask fmt --check` → all files formatted

## Implementation Readiness

- **Spec clarity**: All 11 file changes enumerated with exact line numbers and before/after text
- **Change order**: Sequential with verification after each step
- **Test coverage**: Existing tests validate the refactor (no new tests needed)
- **Documentation**: Internal CLAUDE.md and external LSP_IMPLEMENTATION_GUIDE.md remain accurate

## Future Work

After this PR merges:

1. **Follow-up #1**: Evaluate `crates/perl-parser/src/ide/lsp_compat/` (5084 LOC) — still needed?
2. **Follow-up #2**: Audit perl-lsp startup path — ensure no LSP features depend indirectly on parser's re-exports
3. **Follow-up #3**: Update module diagrams in docs/reference/ARCHITECTURE.md

These are explicitly deferred to preserve this PR's scope.

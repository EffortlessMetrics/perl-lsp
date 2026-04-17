# ADR-001: Diagnostic Stability — Staggered Two-PR Implementation

## Status
Proposed

## Context

Users experience "flaky" diagnostics during active editing — errors appearing and disappearing as they type. Investigation (research_analysis, verification_agent, plan_review, maintainer_vision_agent) identified two root causes:

1. **`publish_parse_errors_fast` has no staleness guard** — intermediate parse states (e.g., `my $x = `) are published even when a newer document version has already arrived.
2. **`IncrementalDocument` AST is built but unused** — `Parser::new().parse()` runs on every keystroke despite `IncrementalDocument::apply_edits()` being called and producing an incrementally-parsed AST.

Both issues cause diagnostic flicker, but they differ fundamentally in implementation complexity and risk.

## Decision

Split the fix into two independent PRs:

### PR 1: Generation-Aware Publication Guard (ships immediately)

Add a generation-snapshot check to `publish_parse_errors_fast` in `diagnostics.rs`, mirroring the existing guard in `publish_diagnostics` at lines 495–506. When a newer document version has advanced the generation during the fast-path computation, skip publishing stale diagnostics.

**Why this PR first:**
- Low risk — pure additive guard, mirrors existing pattern
- High value — eliminates diagnostic flicker from generation races
- No API changes required
- Independent of Phase 1

### PR 2: Incremental AST Reuse (deferred)

Use `incremental_doc.root` as the document AST when `IncrementalDocument::apply_edits()` succeeds, replacing the full `Parser::new().parse()` call on every keystroke.

**Why deferred:**
- `IncrementalDocument::apply_edits()` returns `ParseResult<()>` — errors are **silently discarded** on success
- Using `inc.root` without fixing the error-exposure API would **lose all parse error diagnostics** for that edit cycle
- Requires API design work in `perl-incremental-parsing` to expose `ParseResult<Vec<ParseError>>`
- Even after API fix, semantic analysis consistency across reused subtrees needs separate verification
- Becomes its own design issue with dedicated investigation

## Alternatives Considered

### Alternative 1: Single PR with both phases
- **Rejected because**: Phase 1 is blocked on `perl-incremental-parsing` API changes that require their own design and review. Shipping Phase 2 is valuable independently and should not wait.

### Alternative 2: Only fix generation guard (drop incremental AST entirely)
- **Rejected because**: The full reparse on every keystroke is a real performance problem. For large files, users experience 50–100ms of parsing per keystroke. The incremental AST infrastructure exists and should be used.

### Alternative 3: Increase debounce to hide the flicker
- **Rejected because**: Increases perceived lag rather than fixing the root cause. Users would rather see accurate diagnostics quickly than delayed diagnostics.

## Consequences

### Benefits
- Diagnostic flicker eliminated for generation races (PR 1)
- Path cleared for incremental AST reuse once API is designed (PR 2)
- Each PR is independently reviewable and reversible
- PR 1 provides immediate user-facing improvement

### Tradeoffs / Risks
- PR 1 only partially addresses the issue — full-reparse performance still occurs
- PR 2 requires `perl-incremental-parsing` API changes that must be designed carefully to preserve error diagnostics
- Split PRs mean two review cycles

## Implementation Notes

### PR 1 (Generation Guard)
- File: `crates/perl-lsp/src/runtime/diagnostics.rs`
- Target function: `publish_parse_errors_fast` (line 548)
- Pattern: Mirror the existing guard in `publish_diagnostics` (lines 495–506)
- Capture `generation` at snapshot time; compare before publishing; return early if stale

### PR 2 (Incremental AST — future)
- Files: `text_sync.rs` (primary), `perl-incremental-parsing/src/incremental/incremental_document.rs` (API)
- Must expose parse errors from `apply_edits()` before AST reuse can proceed
- Semantic analysis consistency must be verified separately

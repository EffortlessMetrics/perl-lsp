# Casebook

Exhibit PRs that demonstrate the development model and key capabilities.

## Methodology

This casebook follows the **quality-first** forensics approach:

- **Quality surfaces** are primary: Maintainability, Correctness, Governance, Reproducibility
- **Budget metrics** are secondary, always with provenance
- **Every metric** carries: value, kind, basis, coverage, confidence

See the methodology docs:
- [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md) - Decision-weighted DevLT method
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema
- [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) - The four quality surfaces
- [`ANALYZER_FRAMEWORK.md`](ANALYZER_FRAMEWORK.md) - Specialist analyzers

## How to Read This

Each exhibit shows:
- **What it proves** (1 line)
- **Review map** (key files/surfaces touched)
- **Proof bundle** (receipts: test output, gate output, benchmarks)
- **What went wrong → fix → prevention** (if applicable)
- **Quality deltas** (+2/+1/0/-1/-2 per surface)
- **Budget** (DevLT range + provenance, compute with basis)

---

## Exhibits

### Exhibit 1: Semantic Analyzer Phase 1 (Issue #188 → PRs #231/232/234)

**What it proves:** Major capability additions can be delivered cleanly with explicit receipts and phased architecture.

**Review map:**
- `crates/perl-parser/src/semantic.rs` (+183 lines, 12 handlers)
- `crates/perl-parser/src/semantic/model.rs` (SemanticModel API)
- `crates/perl-parser/tests/semantic_smoke_tests.rs` (+580 lines)
- 3 design docs totaling 1765 lines

**Proof bundle:**
- `just ci-gate`: 274/274 tests passing
- 35 total tests (27 active, 8 Phase 2/3 feature-gated)
- Merge checklist with explicit test commands

**Scar story:** N/A - Clean PR with no drift detected.

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +2 | SemanticModel API creates clean boundary |
| Correctness | +1 | 35 tests, feature-gated phases |
| Governance | +1 | Merge checklist pattern established |
| Reproducibility | +1 | Explicit gate commands in PR |

**Factory delta:**
- Added SemanticModel API as canonical LSP entry point
- Established merge checklist pattern for future capability PRs
- Introduced `semantic-phase2` feature flag for incremental enhancement

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 60–90m | estimated; github_only; medium; 4 design decisions, 0 friction |
| CI | ~12m | estimated; local gate |
| LLM | ~8 units | estimated; 4 implementation iterations |

**Exhibit score:** 4.8/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 4, Efficiency: 5)

**Dossier:** [`forensics/pr-231-232-234.md`](forensics/pr-231-232-234.md)

---

### Exhibit 2: Substitution Operator Correctness (PR #260 + #264)

**What it proves:** Mutation testing catches real bugs that would cause silent production failures.

**Review map:**
- `crates/perl-parser/src/parser/quote_parser.rs` (validation hardening)
- `crates/perl-lexer/src/lib.rs` (delimiter pairing)
- `crates/perl-parser/tests/quote_parser_mutation_hardening.rs`

**Proof bundle:**
- Mutant IDs: MUT_002 (mixed-delimiter), MUT_005 (invalid modifiers)
- Ignored test baseline: `bug=8` → `bug=4` (4 bugs fixed)
- Regression tests: `test_ac2_empty_replacement_balanced_delimiters`, `test_ac2_invalid_flag_combinations`

**Scar story:**
- **Wrong:** Parser accepted `s/foo/bar/z` (invalid modifier) and failed on `s[pat]{repl}` (mixed delimiters)
- **How caught:** Mutation testing survived mutants exposed validation gaps
- **Fix:** Added `extract_substitution_parts_strict()`, explicit `paired_closing()` helper
- **Prevention:** Mutation-killing tests added, "lex liberally, parse strictly" pattern established

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +1 | Validation boundary clarified (lexer vs parser) |
| Correctness | +2 | 4 bugs fixed, mutation-hardened |
| Governance | +1 | Mutation testing pattern established |
| Reproducibility | 0 | No change |

**Factory delta:**
- MUT_002 and MUT_005 now killed
- Data-driven test pattern reduced ~140 lines while improving coverage
- Architectural pattern: validation moved from lexer to parser for better errors

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 60–90m | estimated; github_only; medium; 3 decisions, 2 friction (mutant analysis) |
| CI | ~10m | estimated; local gate |
| LLM | ~6 units | estimated; 3 iterations |

**Exhibit score:** 5/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 5, Efficiency: 5)

**Dossier:** [`forensics/pr-260-264.md`](forensics/pr-260-264.md)

---

### Exhibit 3: Test Harness Hardening (PR #251 + #252 + #253)

**What it proves:** Systematic infrastructure fixes eliminate entire classes of flakiness rather than patching individual tests.

**Review map:**
- `crates/perl-lsp/tests/*.rs` (test harness)
- `crates/perl-lsp/src/errors.rs` (new error codes)
- `crates/perl-lsp/src/lsp/server_impl/dispatch.rs` (shutdown handling)

**Proof bundle:**
- Ignored test baseline: `brokenpipe=386` → `brokenpipe=0` (100% elimination)
- Total ignored: 580 → 215 (-365 tests unignored)
- 123 tests unignored in single PR (#251)

**Scar story:**
- **Wrong:** Tests failed with BrokenPipe because they didn't send `initialized` notification (LSP spec requirement) and didn't call shutdown before exit
- **How caught:** Systematic analysis of ignore categories revealed protocol violation pattern
- **Fix:** Added `initialized` notification to all tests, `shutdown_initiated` atomic flag, proper shutdown sequence
- **Prevention:** New error codes (`CONNECTION_CLOSED`, `TRANSPORT_ERROR`), JSON-RPC compliance function, test pattern requiring explicit `shutdown_and_exit()`

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +2 | Test harness now enforces protocol compliance |
| Correctness | +2 | 365 tests unignored, proper error handling |
| Governance | +1 | Protocol compliance pattern established |
| Reproducibility | +1 | Tests now deterministic |

**Factory delta:**
- Test harness now enforces LSP protocol compliance
- New error handling infrastructure for graceful degradation
- Pattern established: all tests must call `shutdown_and_exit()` explicitly

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 90–130m | estimated; github_only; medium; 4 decisions, 3 friction (protocol debugging) |
| CI | ~15m | estimated; local gate |
| LLM | ~10 units | estimated; 5 iterations across 3 PRs |

**Exhibit score:** 5/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 5, Efficiency: 5)

**Dossier:** [`forensics/pr-251-252-253.md`](forensics/pr-251-252-253.md)

---

### Exhibit 4: Name Span for LSP Navigation (Issue #181 → PR #259)

**What it proves:** Precise LSP navigation requires name_span infrastructure to place cursor on identifiers rather than full blocks.

**Review map:**
- `crates/perl-parser/src/lsp/call_hierarchy_provider.rs` (+49/-19, `selection_range_from_name_span()` helper)
- `crates/perl-parser/src/ast.rs` (`phase_span: Option<SourceLocation>` added to PhaseBlock)
- `crates/perl-parser/src/parser.rs` (name_span capture from tokens)
- `crates/perl-parser/tests/name_spans_special_test.rs` (+342 lines, 11 tests)

**Proof bundle:**
- `cargo test -p perl-parser --test name_spans_special_test`: 11/11 passing
- Tests cover: AUTOLOAD, DESTROY, BEGIN, END, CHECK, INIT, UNITCHECK spans
- Byte-precise assertions verify exact span boundaries

**Scar story:** N/A - Clean PR with no drift detected.

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +1 | `selection_range_from_name_span()` helper reusable |
| Correctness | +1 | 11 byte-precise span tests |
| Governance | 0 | LSP 3.17 compliance (existing standard) |
| Reproducibility | 0 | No change |

**Factory delta:**
- `selectionRange` now correctly identifies symbol name portion (LSP 3.17 compliant)
- `phase_span` added to PhaseBlock AST node for phase block keywords
- Container names added to workspace symbols for disambiguation

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 45–75m | estimated; github_only; medium; 3 decisions, 0 friction |
| CI | ~8m | estimated; local gate |
| LLM | ~4 units | estimated; 2 iterations |

**Exhibit score:** 4.8/5 (Clarity: 5, Scope: 4, Evidence: 5, Tests: 5, Efficiency: 5)

**Dossier:** [`forensics/pr-259.md`](forensics/pr-259.md)

---

### Exhibit 5: Statement Tracker + Heredoc Block-Aware (PRs #225/226/229)

**What it proves:** Multi-PR feature decomposition enables incremental delivery with independent testability.

**Review map:**
- `crates/tree-sitter-perl-rs/src/statement_tracker.rs` (+566 lines, StatementTracker API)
- `crates/tree-sitter-perl-rs/src/heredoc_parser.rs` (HeredocScanner integration)
- `crates/perl-parser/tests/sprint_a_heredoc_ast_tests.rs` (+236 lines, F5/F6 fixtures)

**Proof bundle:**
- 28 total tests: 14 unit (StatementTracker API), 8 integration (F1-F4), 6 AST-level (F5-F6)
- F1-F6 fixtures document expected behavior for block scenarios
- Sprint A completion milestone achieved with PR #229

**Scar story:** N/A - Clean PRs with no drift detected.

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +2 | StatementTracker clean API, modular design |
| Correctness | +1 | 28 tests with fixtures |
| Governance | +1 | Sprint milestone pattern established |
| Reproducibility | +1 | F1-F6 fixture documentation |

**Factory delta:**
- Block-aware statement end detection: `find_statement_end_line()` handles semicolon vs. expression heredocs
- `HeredocContext` captures `block_depth_at_declaration` for future semantic analysis
- `scan_for_test()` exposes internal state for validation

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 60–90m | estimated; github_only; medium; 4 decisions, 0 friction |
| CI | ~10m | estimated; local gate across 3 PRs |
| LLM | ~8 units | estimated; 4 iterations across 3 PRs |

**Exhibit score:** 5/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 5, Efficiency: 5)

**Dossier:** [`forensics/pr-225-226-229.md`](forensics/pr-225-226-229.md)

---

### Exhibit 6: Parser/LSP Modularization + wasm/pull diagnostics stabilization (PR #294)

**What it proves:** Large-scale structural refactors can land safely when paired with shims, strict gating, and regression tests that lock client-visible behavior.

**Review map:**
- `crates/perl-parser/src/parser/*` (parser monolith split)
- `crates/perl-parser/src/lsp/features/*` (feature regrouping)
- wasm gating + stubs (`formatting`, `completion`, `config`, walkdir target dep)
- pull diagnostics wiring + caching behavior (`pull.rs`, new pull tests)

**Proof bundle:**
- `cargo check -p perl-parser --target wasm32-unknown-unknown`: passes
- `cargo test -p perl-parser --test pull_diagnostics_tests`: Full → Unchanged invariant
- `cargo test -p perl-parser`: full host test suite

**Scar story:**
- **Wrong:** wasm surface drift (FS/process APIs) + pull diagnostics API mismatch (lsp-types 0.97) + missing Full→Unchanged invariant
- **How caught:** wasm32 compilation failures exposed process/FS dependencies; diagnostics tests exposed API drift
- **Fix:** explicit wasm stubs/target deps + API-aligned report construction + Full→Unchanged regression test
- **Prevention:** test locks flicker behavior; hard boundary for wasm-only functionality

**Quality deltas:**

| Surface | Delta | Notes |
|---------|-------|-------|
| Maintainability | +2 | Modular parser + LSP feature tree |
| Correctness | +1 | Diagnostics behavior regression test; wiring fixes |
| Governance | +1 | Shims + gating playbook |
| Reproducibility | +1 | Explicit wasm + targeted test commands |

**Factory delta:**
- wasm32 compilation now validated in gate
- `lsp-types` made optional via `lsp-compat` feature flag
- Pull diagnostics Full→Unchanged invariant locked by regression test
- walkdir moved to target-specific dependency

**Budget:**

| Metric | Value | Provenance |
|--------|-------|------------|
| DevLT | 90–130m | estimated; github_only; medium; 5 decisions, 3 friction (wasm debugging) |
| CI | ~15m | estimated; local gate |
| LLM | ~10 units | estimated; 5 iterations |

**Exhibit score:** 4.8/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 4, Efficiency: 5)

**Related commits:**
- `f751c622` - Refactor Parser & LSP Architecture + Add Workspace Features (#294)
- `7834d191` - LSP server capabilities and utilities
- `cfa64c5c` - wasm32 target support

---

## What These Exhibits Demonstrate

### AI-Native Development Patterns

1. **Receipts over claims**: Every capability/fix has test output, gate output, or baseline metrics
2. **Wrongness is recorded**: When mutation testing or protocol analysis finds bugs, the scar story is documented
3. **Factory deltas compound**: Each PR adds guardrails that prevent recurrence
4. **Phased delivery**: Large features use feature flags for incremental enhancement
5. **Quality first**: Quality deltas are primary; budget is secondary with provenance

### Quality Summary

| Exhibit | Maint | Correct | Gov | Repro | Total |
|---------|-------|---------|-----|-------|-------|
| #231/232/234 Semantic | +2 | +1 | +1 | +1 | +5 |
| #260/264 Substitution | +1 | +2 | +1 | 0 | +4 |
| #251-253 Harness | +2 | +2 | +1 | +1 | +6 |
| #259 Name Span | +1 | +1 | 0 | 0 | +2 |
| #225/226/229 Statement Tracker | +2 | +1 | +1 | +1 | +5 |
| #294 Parser/LSP Modularization | +2 | +1 | +1 | +1 | +5 |

**Aggregate quality delta**: +27 across 6 exhibits

### Combined Budget (with Provenance)

| Exhibit | DevLT | CI | LLM | Quality Achieved |
|---------|-------|----|----|------------------|
| #231/232/234 Semantic | 60–90m | ~12m | ~8 | 12/12 handlers, clean API |
| #260/264 Substitution | 60–90m | ~10m | ~6 | 4 bugs fixed, mutation-hardened |
| #251-253 Harness | 90–130m | ~15m | ~10 | 365 tests unignored |
| #259 Name Span | 45–75m | ~8m | ~4 | 11 span tests, LSP 3.17 |
| #225/226/229 Statement Tracker | 60–90m | ~10m | ~8 | 28 tests, F1-F6 fixtures |
| #294 Parser/LSP Modularization | 90–130m | ~15m | ~10 | wasm32 gate, lsp-compat feature |

**Total DevLT**: 405–605m (estimated; github_only; medium confidence)

**Coverage**: All estimates are `github_only` (no agent logs available for retrospective analysis).

**Basis**: Decision events + friction events per exhibit, weighted per [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md).

---

## Adding Exhibits

To add an exhibit:
1. Use Level 2 archaeology from [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md)
2. Identify the "what it proves" in one line
3. Document the review map (key files)
4. Link to receipts (test output, gate output, benchmarks)
5. Record any wrongness discovered → fix → prevention
6. **Add quality deltas** using the four surfaces from [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md)
7. **Estimate budget with provenance** using [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md)
8. Create a dossier in [`forensics/`](forensics/) if one doesn't exist

See [`forensics/INDEX.md`](forensics/INDEX.md) for the PR inventory.

## Methodology Documentation

- [`DEVLT_ESTIMATION.md`](DEVLT_ESTIMATION.md) - Decision-weighted DevLT method
- [`METRICS_PROVENANCE.md`](METRICS_PROVENANCE.md) - Provenance schema for all metrics
- [`QUALITY_SURFACES.md`](QUALITY_SURFACES.md) - The four quality surfaces
- [`ANALYZER_FRAMEWORK.md`](ANALYZER_FRAMEWORK.md) - Specialist analyzer specs
- [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md) - Full dossier template

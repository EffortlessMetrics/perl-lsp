# Casebook

Exhibit PRs that demonstrate the development model and key capabilities.

## How to Read This

Each exhibit shows:
- **What it proves** (1 line)
- **Review map** (key files/surfaces touched)
- **Proof bundle** (receipts: test output, gate output, benchmarks)
- **What went wrong → fix → prevention** (if applicable)
- **DevLT band** (0-10m / 10-30m / 30m+)
- **Compute band** (unknown / low / med / high)

---

## Exhibits

### Exhibit 1: Semantic Analyzer Phase 1 (PR #188)

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

**Factory delta:**
- Added SemanticModel API as canonical LSP entry point
- Established merge checklist pattern for future capability PRs
- Introduced `semantic-phase2` feature flag for incremental enhancement

**DevLT:** 60-90m | **Compute:** moderate

**Exhibit score:** 4.8/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 4, Efficiency: 5)

**Dossier:** [`forensics/pr-188.md`](forensics/pr-188.md)

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

**Factory delta:**
- MUT_002 and MUT_005 now killed
- Data-driven test pattern reduced ~140 lines while improving coverage
- Architectural pattern: validation moved from lexer to parser for better errors

**DevLT:** 60-90m | **Compute:** moderate

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

**Factory delta:**
- Test harness now enforces LSP protocol compliance
- New error handling infrastructure for graceful degradation
- Pattern established: all tests must call `shutdown_and_exit()` explicitly

**DevLT:** 90-120m | **Compute:** moderate

**Exhibit score:** 5/5 (Clarity: 5, Scope: 5, Evidence: 5, Tests: 5, Efficiency: 5)

**Dossier:** [`forensics/pr-251-252-253.md`](forensics/pr-251-252-253.md)

---

## What These Exhibits Demonstrate

### AI-Native Development Patterns

1. **Receipts over claims**: Every capability/fix has test output, gate output, or baseline metrics
2. **Wrongness is recorded**: When mutation testing or protocol analysis finds bugs, the scar story is documented
3. **Factory deltas compound**: Each PR adds guardrails that prevent recurrence
4. **Phased delivery**: Large features use feature flags for incremental enhancement

### Combined Budget Efficiency

| Exhibit | DevLT | Compute | Quality Achieved |
|---------|-------|---------|------------------|
| #188 Semantic | 60-90m | moderate | 12/12 handlers, clean API |
| #260/264 Substitution | 60-90m | moderate | 4 bugs fixed, mutation-hardened |
| #251-253 Harness | 90-120m | moderate | 365 tests unignored |

Total DevLT: ~4-5 hours for significant capability + quality improvements.

---

## Adding Exhibits

To add an exhibit:
1. Use Level 2 archaeology from [`FORENSICS_SCHEMA.md`](FORENSICS_SCHEMA.md)
2. Identify the "what it proves" in one line
3. Document the review map (key files)
4. Link to receipts (test output, gate output, benchmarks)
5. Record any wrongness discovered → fix → prevention
6. Estimate DevLT and compute bands
7. Create a dossier in [`forensics/`](forensics/) if one doesn't exist

See [`forensics/INDEX.md`](forensics/INDEX.md) for the PR inventory.

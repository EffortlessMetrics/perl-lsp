# Context: #4429 — Wave E Microcrate Collapse

## Decision Log

### Crate Naming: `perl-diagnostics` vs `perl-diagnostic-catalog`
**Decision:** `perl-diagnostics` (family-noun pattern, consistent with `perl-module`, `perl-workspace`)

**Why:** Orchestrator-ruled decision from plan-reviewer. Family-noun (plural for semantic family) is clearer than adjective-noun compound. Aligns with Wave 1 naming (`perl-module` not `perl-module-facade`).

**Alternative rejected:** `perl-diagnostic-catalog` — too wordy; -catalog suffix is internal module name, not public crate name.

### Module Layout: Flat vs Nested
**Decision:** Flat layout: `codes/`, `types/`, `catalog/` as sibling modules in `src/`

**Why:** Each source crate becomes exactly one module. Simple cross-references via `crate::codes::DiagnosticCode`, etc. No nested hierarchy needed.

**Alternative considered:** Nested like `src/subsystems/codes/mod.rs` — rejected for simplicity; flat is sufficient.

### Re-export Strategy: Explicit vs Wildcard
**Decision:** Explicit per-symbol re-exports via `src/api.rs` (NO wildcards)

**Why:** `DiagnosticSeverity` and `DiagnosticTag` are defined in BOTH `codes/` and `types/` modules. Wildcard re-exports (`pub use crate::codes::*;` + `pub use crate::types::*;`) create compile error "ambiguous reexports". Explicit list solves this.

**Code pattern:**
```rust
// src/api.rs
pub use crate::codes::{DiagnosticCode, DiagnosticSeverity, DiagnosticTag};
pub use crate::types::{Diagnostic, DiagnosticSeverity as TypesDiagnosticSeverity, /* ... */};
pub use crate::catalog::{diagnostic_meta, parse_error, syntax_error, /* all public fns */};
```

**Alternative rejected:** Wildcard re-exports — cause compile error. Aliasing in re-export (e.g., `DiagnosticSeverity as CodesDiagnosticSeverity`) — creates confusion; better to document duplication and unify in v0.15.0.

### Type Unification: Solve Now vs Defer
**Decision:** Defer to v0.15.0 (major version bump); document in README

**Why:** Both `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` are identical by semantics (Error=1, Warning=2, Information=3, Hint=4). Merging them now requires breaking change for consumers that explicitly reference `types::` path. Wave E is shape-preserving (move-only), not breaking. Full unification will happen in v0.15.0 clean-break release.

**How documented:** README.md includes section "Type Duplication and Future Unification" explaining:
- Why both types exist (from separate original crates)
- They are semantically identical
- Recommendation to use `codes::DiagnosticSeverity` as canonical (older, more established)
- Future v0.15.0 will remove duplicate and re-export single canonical type

**Alternative rejected:** Unify now by removing `types::` versions — breaks Wave E shape-preservation principle and creates coupling with v0.13.0 release.

### Feature Flags: Keep or Rationalize
**Decision:** Keep `serde` feature (optional, gated on both codes and types modules)

**Why:** Original crates have `serde` feature for optional serialization. New crate should preserve this for consumers that may want JSON serialization of diagnostics. No rationalization needed; straightforward port.

### Publish Allowlist Position: Tier 3 vs Tier 5
**Decision:** Tier 3 (analysis and indexing)

**Why:** Original crates were spread across tiers. New unified crate consolidates diagnostic subsystem and should sit alongside other analysis-layer crates (`perl-semantic-analyzer`, `perl-lsp-diagnostics`). Tier 3 is correct placement.

---

## Objections Addressed

### O1: Type duplication is deferred, not solved
**Objection:** Keeping both `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` ships technical debt.

**Resolution:** Acknowledged. Duplication is intentional Wave E constraint (shape-preserving, no breaking changes). Full unification (removing one type, re-exporting the other) deferred to v0.15.0 major release. Documented explicitly in README and inline deprecation notices. Risk is LOW — duplication is semantic (both represent same LSP values), has no runtime impact, and consumers have clear guidance (use `codes::` variant).

**Evidence:** ADR-0041 permits shape-preserving moves in Wave E without requiring semantic changes. v0.15.0 is scheduled for next cycle where breaking changes are acceptable.

### O2: Why not use type aliases to unify DiagnosticSeverity?
**Objection:** Type aliases could present a unified name while keeping original implementations.

**Resolution:** Type aliases (e.g., `pub type DiagnosticSeverity = codes::DiagnosticSeverity;`) would work but introduce coupling between modules and mask the duplication. Better to be explicit: both types exist, are identical, will be unified later. Consumers can choose which to use. Aliases defer the cleanup burden to users of the crate.

### O3: Wildcard re-exports would be simpler
**Objection:** Just use `pub use crate::codes::*;` and `pub use crate::types::*;` — cleaner than explicit lists.

**Resolution:** Tested approach: Wildcard re-exports cause compile error "ambiguous reexports" due to `DiagnosticSeverity` and `DiagnosticTag` defined in both modules. Rust compiler rejects this. Explicit re-export list is the only safe pattern. Verbosity is a small price for correctness.

### O4: Consumers will be confused by path changes (e.g., `perl_diagnostics_codes::` → `perl_diagnostic_catalog::codes::`)
**Objection:** Changing import paths from `use perl_diagnostics_codes::DiagnosticCode;` to `use perl_diagnostic_catalog::codes::DiagnosticCode;` is a breaking change for external consumers.

**Resolution:** This is a "published crate" in allowlist, so semantic versioning applies. Changing the crate's public module structure (adding the `codes::` prefix) is indeed a breaking change, but that's acceptable for a v0.13.0 release per ADR-0041's clean-break timeline. External consumers must update imports. Internal consumers (perl-lsp ecosystem) are already being updated by this PR. Documentation of migration path should be provided in v0.13.0 release notes.

---

## Research Findings

### Verified Claims
1. **LSP DiagnosticSeverity mapping** (from research-verifier): Error=1, Warning=2, Information=3, Hint=4 — ✓ Confirmed in LSP 3.17 spec
2. **Original crate locations and dependencies** (from accuracy-scout): All 3 crates exist at claimed paths with claimed dependencies — ✓ Verified
3. **Workspace member count** (from accuracy-scout): 122 → 120 post-Wave-E — ✓ Verified (122 in current state, this PR removes 3, adds 1)

### No External Blockers
- No Perl feature claims to verify (Wave E is pure refactoring)
- No LSP spec compliance issues (diagnostic types unchanged, only code organization)
- No CLI/API contract changes (all public types preserved, paths change)

---

## Related Issues & PRs

### Tracking Issues
- **#4410** (microcrate collapse master tracking) — this Wave E is scoped in the master issue
- **ADR-0041** (docs/adr/0041-microcrate-collapse.md) — policy authority for Wave E scope and naming

### Related Waves
- **Wave 1** (#4422, merged) — perl-module-* → perl-module (pilot; established pattern for this work)
- **Wave A** (#4426, in-build) — perl-workspace-* → perl-workspace (parallel work; independent)
- **Waves F–H** (deferred) — LSP provider cleanup (scheduled after Waves 1–5, E complete)

### Follow-up Work
- **Ledger amendment** — separate PR to update `.spec/microcrate-collapse/ledger.md` to reflect Wave E completion (not in scope of this implementation)
- **v0.13.0 release notes** — migration guide for external consumers using old crate names (post-implementation, release phase)
- **v0.15.0 planning** — type unification work (major release; not in scope now)

---

## Architecture Notes

### Dependency Graph
- New `perl-diagnostic-catalog` is a **Tier 3** leaf crate (no internal workspace dependencies beyond itself)
- Sits above Tier 1–2 (primitives, AST, tokens) and below Tier 4–5 (LSP providers, application)
- Consumers: `perl-lsp-code-actions`, `perl-lsp-diagnostics`, `perl-lsp` (server)
- No consumers depend on internal modules directly; all go through public API

### Compatibility
- This is a **shape-preserving move** of three crates into one
- All public types, functions, and enums preserved
- Only change: crate name and module paths for imports
- No behavior changes; no new features; no API extensions

---

## Test Strategy

### Test Files to Migrate (6 total)
1. From `perl-diagnostics-codes/tests/`:
   - `comprehensive_unit_tests.rs` → `codes_comprehensive_unit_tests.rs`
   - `context_hint_tests.rs` → `codes_context_hint_tests.rs`
   - `diagnostic_code_completeness.rs` → `codes_diagnostic_code_completeness.rs`

2. From `perl-lsp-diagnostic-catalog/tests/`:
   - `catalog_coverage.rs` → `catalog_coverage.rs` (keep name, update imports)
   - `context_hint_catalog_tests.rs` → `catalog_context_hint_tests.rs`

3. From `perl-lsp-diagnostic-types/tests/`:
   - `comprehensive_unit_tests.rs` → `types_comprehensive_unit_tests.rs`

### Naming Convention
- Prefix test files with module name: `codes_*`, `catalog_*`, `types_*`
- Prevents collisions in unified test directory
- Makes test provenance clear (which original crate did this test come from?)

### Coverage
- All original tests preserved (no tests deleted)
- All import paths updated to match new module structure
- No new tests added in this wave (shape-preserving)

---

## Known Limitations & Deferred Work

1. **Type duplication** — `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` both present. Will be unified in v0.15.0.
2. **Ledger amendment** — `.spec/microcrate-collapse/ledger.md` amendment is separate follow-up PR (not in scope of implementation).
3. **Documentation updates** — Only new crate's README included. General "migration guide" docs deferred to v0.13.0 release phase.
4. **No semantic analysis improvements** — This is move-only, not an opportunity to refactor diagnostic logic.

---

## Success Criteria (from Plan-Reviewer)

- New crate compiles and all tests pass
- All 3 old crates deleted
- 4 consumer crates updated with no regressions
- Workspace member count: 122 → 120
- Publish allowlist count: 120 → 118
- No breaking changes to public API (only crate/module names change)
- Documentation exists explaining type duplication and unification path


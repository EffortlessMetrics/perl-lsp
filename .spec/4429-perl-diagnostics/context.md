# Context: #4429 — Wave E Microcrate Collapse

## Decision Log

### Crate Naming: `perl-diagnostics` (orchestrator-locked)
**Decision:** `perl-diagnostics` (family-noun pattern, consistent with `perl-module`, `perl-workspace`, `perl-symbol`)

**Why:** Orchestrator-ruled decision locked by plan-reviewer (see #4429 plan-review comment). Family-noun (plural for semantic family) is clearer than adjective-noun compound. Aligns with Wave 1 pilot naming (`perl-module` not `perl-module-facade`) and Wave A (`perl-workspace`).

**Alternative rejected:** `perl-diagnostic-catalog` — descriptive compound, diverges from the family-noun pattern. The ledger row at `.spec/microcrate-collapse/ledger.md:149` still lists `perl-diagnostic-catalog`; it is stale and will be amended in a follow-up docs PR (same pattern as #4427 for perl-workspace rename). **Do not block this work on the ledger amendment.**

### Module Layout: Flat vs Nested
**Decision:** Flat layout: `codes/`, `types/`, `catalog/` as sibling modules in `src/`

**Why:** Each source crate becomes exactly one module. Simple cross-references via `crate::codes::DiagnosticCode`, etc. No nested hierarchy needed.

**Alternative considered:** Nested like `src/subsystems/codes/mod.rs` — rejected for simplicity; flat is sufficient.

### Re-export Strategy: Explicit (no wildcards)
**Decision:** Explicit per-symbol re-exports via `src/api.rs` (NO wildcards)

**Why:** Wildcard re-exports (`pub use crate::codes::*;` + `pub use crate::types::*;`) would collide on symbols re-exported from both modules. Even after type unification, wildcards are structurally dangerous — a future edit that adds a colliding name in either module would silently break downstream consumers. Explicit per-symbol list is the only safe pattern.

**Code pattern:**
```rust
// src/api.rs — explicit only, no wildcards
pub use crate::codes::{DiagnosticCode, DiagnosticCategory, DiagnosticSeverity, DiagnosticTag};
pub use crate::types::{Diagnostic, RelatedInformation};
// DiagnosticSeverity and DiagnosticTag are canonically defined in codes::
// and re-exported via types::. api.rs re-exports them via the canonical codes:: path.
pub use crate::catalog::{
    DiagnosticMeta, diagnostic_meta, parse_error, syntax_error,
    unexpected_eof, missing_strict, missing_warnings, unused_var, undefined_var,
    missing_package_declaration, duplicate_package, duplicate_sub, missing_return,
    bareword_filehandle, two_arg_open, implicit_return, eval_error_flow,
    critic_severity_5, critic_severity_4, critic_severity_3, critic_severity_2,
    critic_severity_1, from_message,
};
```

**Alternative rejected:** Wildcard re-exports — structurally dangerous even after unification.

### Type Unification: Unify NOW (orchestrator-locked)
**Decision:** Unify `DiagnosticSeverity` and `DiagnosticTag` in this wave. Canonical definitions live in `codes/` module. `types/mod.rs` does `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag}` so that `Diagnostic::severity: DiagnosticSeverity` continues to resolve via the `types` path while sharing a single underlying type.

**Why:** Orchestrator-ruled decision locked by plan-reviewer (see #4429 plan-review comment). `codes::DiagnosticSeverity` is a strict trait-surface superset of `types::DiagnosticSeverity` — it derives `PartialEq, Eq, PartialOrd, Ord, Hash`, has `to_lsp_value()`, `Display`, and optional `serde`; the `types::` version only has `PartialEq, Eq, PartialOrd, Ord`. Unifying to `codes::` is a **non-breaking widening** for any consumer that previously used the `types::` variant (they get more traits, not fewer). Wave E is the ideal moment because:

1. The crate boundary is already being broken (import paths change from `perl_diagnostics_codes::` / `perl_lsp_diagnostic_types::` to `perl_diagnostics::codes::` / `perl_diagnostics::types::`). Adding a second breaking change has zero marginal cost for consumers.
2. Leaving the duplicate public type in v0.13.0's external surface would force a migration-guide note and a second break at v0.15.0.
3. The `types::Diagnostic` struct binds `severity: DiagnosticSeverity` to the `types::` variant today. After `types/mod.rs` re-exports from `codes/`, that field binds to the canonical type with no struct-definition change needed.

**Mechanical path (from plan-reviewer comment on #4429):**
1. Keep `codes/mod.rs` with `DiagnosticSeverity` and `DiagnosticTag` unchanged — these are the canonical definitions.
2. In `types/mod.rs`: delete the `DiagnosticSeverity` and `DiagnosticTag` enum definitions. Add `pub use crate::codes::{DiagnosticSeverity, DiagnosticTag};` so the `Diagnostic` struct's `severity` field and any external consumer referencing `perl_diagnostics::types::DiagnosticSeverity` still resolves to the unified canonical type.
3. `Diagnostic` and `RelatedInformation` structs stay in `types/mod.rs` unchanged in shape.
4. Consumer code that used `perl_lsp_diagnostic_types::DiagnosticSeverity` migrates to `perl_diagnostics::types::DiagnosticSeverity` — which is now the same type as `perl_diagnostics::codes::DiagnosticSeverity`. Cross-path assignment compiles.
5. A test file `tests/type_unification.rs` verifies that the two module paths resolve to the same type (cross-path assignment compiles, discriminants match LSP spec values).

**Alternative rejected:** Defer to v0.15.0 — the orchestrator explicitly overruled this. Shipping two public types in v0.13.0 costs a second migration guide and a second break later for no gain.

### Layer-check Rule (new)
**Decision:** Add an xtask layer-check rule: `perl-diagnostics` must NOT depend on any `perl-lsp-*` crate.

**Why:** Prevents future drift where someone adds LSP wire types (`lsp-types`, `tower-lsp`) to the diagnostic kernel. Keeps the crate at Tier 3 (diagnostic surface), strictly below LSP provider tier.

### Feature Flags: Keep `serde` (optional)
**Decision:** Keep `serde` feature (optional, gated on both codes and types modules). `serde_json` remains a required (non-optional) dependency — it is used unconditionally by `catalog/mod.rs`.

**Why:** Original crates have `serde` feature for optional serialization. New crate should preserve this for consumers that may want JSON serialization of diagnostics. `serde_json` is not optional (it is unconditionally used by the catalog module), so do NOT move it behind a feature flag.

### Publish Allowlist Position: Tier 3
**Decision:** Tier 3 (analysis and indexing), inserted alphabetically within the tier.

**Why:** New unified crate consolidates the diagnostic subsystem and sits alongside other analysis-layer crates. Tier 3 matches the diagnostic-surface position in ADR-0041 Amendment 1.

---

## Objections Addressed

### O1: Type duplication (resolved by unifying in this wave)
**Objection:** Earlier spec drafts proposed keeping both `codes::DiagnosticSeverity` and `types::DiagnosticSeverity` as separate types with a note to unify in v0.15.0.

**Resolution:** Orchestrator ruled: unify NOW. `codes::DiagnosticSeverity` is canonical; `types/mod.rs` re-exports it. The objection is fully resolved — there is no duplication after Wave E. See "Type Unification" decision above for the mechanical path.

### O2: Why not use type aliases (in a separate module) to unify?
**Objection:** Type aliases could present a unified name while keeping original implementations.

**Resolution:** Re-exports are cleaner than aliases — they produce exactly one canonical type with the full trait surface of `codes::DiagnosticSeverity`. No aliasing, no masked duplication.

### O3: Wildcard re-exports would be simpler
**Objection:** Just use `pub use crate::codes::*;` and `pub use crate::types::*;`.

**Resolution:** Wildcards are structurally dangerous — future edits that add colliding names in either module would silently break downstream consumers. Explicit per-symbol list is the safe pattern regardless of whether duplicates currently exist.

### O4: Consumers will need to update import paths
**Objection:** Changing import paths from `use perl_diagnostics_codes::DiagnosticCode;` to `use perl_diagnostics::codes::DiagnosticCode;` is a breaking change for external consumers.

**Resolution:** This is a "published crate" in the allowlist, so semantic versioning applies. v0.13.0 is a clean-break release per ADR-0041 — import path changes and type unification both land together. External consumers update imports once. Internal consumers (perl-lsp ecosystem) are already being updated by this PR.

---

## Research Findings

### Verified Claims
1. **LSP DiagnosticSeverity mapping** (from research-verifier): Error=1, Warning=2, Information=3, Hint=4 — confirmed in LSP 3.17 spec.
2. **LSP DiagnosticTag mapping** (from research-verifier): Unnecessary=1, Deprecated=2 — confirmed in LSP 3.17 spec.
3. **Original crate locations and dependencies** (from accuracy-scout): All 3 crates exist at claimed paths with claimed dependencies.
4. **Workspace member count** (from accuracy-scout): 123 current; 121 after Wave E (−3 absorbed, +1 new).
5. **Trait-surface relationship** (from architecture-reviewer): `codes::DiagnosticSeverity` is a strict superset of `types::DiagnosticSeverity` (adds `Hash`, `to_lsp_value()`, `Display`, optional `serde`). Widening to `codes::` is non-breaking.

### No External Blockers
- No Perl feature claims to verify (Wave E is pure refactoring).
- No LSP spec compliance issues (diagnostic type values unchanged).
- No CLI/API contract changes (public types preserved; import paths change).

---

## Related Issues & PRs

### Tracking Issues
- **#4410** (microcrate collapse master tracking) — Wave E is scoped in the master issue.
- **ADR-0041** (docs/adr/0041-microcrate-collapse.md) — policy authority for Wave E scope and naming.

### Related Waves
- **Wave 1** (#4422, merged) — perl-module-* → perl-module (pilot; established the pattern for this work).
- **Wave A** (#4426, in-build) — perl-workspace-* → perl-workspace (parallel work; independent).
- **Waves F–H** (deferred) — LSP provider cleanup (scheduled after Waves 1–5, E complete).

### Follow-up Work
- **Ledger amendment** — separate docs PR to update `.spec/microcrate-collapse/ledger.md:149` row from `perl-diagnostic-catalog` to `perl-diagnostics` (same pattern as #4427 for perl-workspace rename). Not in scope of this implementation.
- **v0.13.0 release notes** — migration guide for external consumers using old crate names and the `types::DiagnosticSeverity` type (post-implementation, release phase).

---

## Architecture Notes

### Dependency Graph
- New `perl-diagnostics` is a **Tier 3** leaf crate (no internal workspace dependencies).
- Sits above Tier 1–2 (primitives, AST, tokens) and below Tier 4–5 (LSP providers, application).
- Consumers: `perl-lsp-code-actions`, `perl-lsp-diagnostics`, `perl-lsp-rs` (server binary in `crates/perl-lsp/`).
- `perl-lsp-diagnostics` stays as its own crate — it is Wave G1 scope, NOT absorbed in Wave E.
- No consumers depend on internal modules directly; all go through public API.

### Compatibility
- This is a **shape-preserving move** for source content, with one intentional semantic change: type unification of `DiagnosticSeverity` and `DiagnosticTag`.
- All public functions and enums preserved. `Diagnostic` and `RelatedInformation` structs unchanged.
- Changes for consumers:
  - Crate name: `perl-diagnostics-codes` / `perl-lsp-diagnostic-types` / `perl-lsp-diagnostic-catalog` → `perl-diagnostics`.
  - Module paths: `perl_diagnostics_codes::` → `perl_diagnostics::codes::`, etc.
  - Type identity: `types::DiagnosticSeverity` is now the same type as `codes::DiagnosticSeverity` (widening, non-breaking).

---

## Test Strategy

### Test Files to Migrate (6 external + 4 inline = 10 total)

**External files:**
1. From `perl-diagnostics-codes/tests/`:
   - `comprehensive_unit_tests.rs` → `codes_comprehensive_unit_tests.rs`
   - `context_hint_tests.rs` → `codes_context_hint_tests.rs`
   - `diagnostic_code_completeness.rs` → `codes_diagnostic_code_completeness.rs`

2. From `perl-lsp-diagnostic-catalog/tests/`:
   - `catalog_coverage.rs` → `catalog_coverage.rs` (keep name, update imports)
   - `context_hint_catalog_tests.rs` → `catalog_context_hint_tests.rs`

3. From `perl-lsp-diagnostic-types/tests/`:
   - `comprehensive_unit_tests.rs` → `types_comprehensive_unit_tests.rs`

**Inline tests (identified by plan-reviewer, missed by initial accuracy pass):**

4. `perl-lsp-diagnostic-catalog/src/lib.rs:169-205` contains 4 inline tests in a `#[cfg(test)] mod tests` block:
   - `parse_error_includes_stable_code_and_docs_url`
   - `critic_codes_have_no_docs_url`
   - `eval_error_flow_has_stable_code_and_docs_url`
   - `message_inference_is_case_insensitive`

   These must migrate to `src/catalog/mod.rs` as an inline `#[cfg(test)] mod tests` block.

**New test file (type unification verification):**

5. `tests/type_unification.rs` — verifies that `perl_diagnostics::codes::DiagnosticSeverity` and `perl_diagnostics::types::DiagnosticSeverity` are the same type (cross-path assignment compiles). Same for `DiagnosticTag`.

### Naming Convention
- Prefix external test files with module name: `codes_*`, `catalog_*`, `types_*`.
- Prevents collisions in unified test directory.
- Makes test provenance clear.

### Coverage
- All original tests preserved (no tests deleted).
- All import paths updated to match new module structure.
- One new test file (`type_unification.rs`) verifies unification invariant.

---

## Known Limitations & Deferred Work

1. **Ledger amendment** — `.spec/microcrate-collapse/ledger.md:149` amendment is a separate follow-up docs PR. The row currently reads `perl-diagnostic-catalog`; correct is `perl-diagnostics`.
2. **Documentation updates** — Only the new crate's README is included here. General "migration guide" docs are deferred to v0.13.0 release phase.
3. **No semantic analysis improvements** — This is a move + type unification; not an opportunity to refactor diagnostic logic.

---

## Success Criteria (from Plan-Reviewer)

- New crate `perl-diagnostics` compiles and all tests pass.
- All 3 old crates deleted.
- 3 consumer crates updated with no regressions (`perl-lsp-code-actions`, `perl-lsp-diagnostics`, `perl-lsp-rs`).
- Workspace member count: 123 → 121.
- Publish allowlist count: 120 → 118.
- No public-API regressions; `types::DiagnosticSeverity` and `types::DiagnosticTag` are unified to their `codes::` canonical definitions via `pub use`.
- `api.rs` uses explicit per-symbol re-exports (no wildcards).
- Layer-check rule added: `perl-diagnostics` forbidden from depending on `perl-lsp-*`.

# Context: #4541 — Wave Final PR B (LSP Deferrals Absorption)

## Executive summary

PR B is the second of two Wave Final PRs. PR A (Wave 4-Completion, #4542 merged 2026-04-21) absorbed 3 parser satellites into `perl-parser`. PR B absorbs 3 LSP deferrals (feature-catalog, config, framing) into `perl-lsp-rs-core`, reducing published count from 34 → 31. This closes out the Amendment 6 target (31 published + 4 internal).

---

## Decision log

### D4: perl-feature-catalog → perl-lsp-rs-core::feature_catalog (build-dep)

**Decision:** Absorb into `perl-lsp-rs-core` as internal build-time utility module.

**Why:** Feature-catalog is a build-time codegen utility with zero runtime dependency. Actual consumers are `perl-dap/build.rs` and `perl-lsp-rs-core/build.rs` — both building `perl-lsp-rs-core` itself or dependent crates. Absorbing consolidates build infrastructure into the core LSP runtime crate.

**Original ledger error:** Prior agents listed perl-parser as absorption target. Investigation found parser has zero dependency on feature-catalog. Corrected to perl-lsp-rs-core per plan-review Correction 1.

**Rejects:**
- Keep as standalone published crate — conflicts with Amendment 6 trim targets
- Absorb into perl-dap — creates asymmetry (perl-dap should not own build infrastructure for perl-lsp-rs-core)

---

### D5: perl-lsp-config → perl-lsp-rs-core::config (break cycle via platform extraction)

**Decision:** Extract `resolve_perl_path_with_toolchain`, `detect_perlbrew_perl`, `detect_plenv_perl` from `perl-dap::platform` into new `perl-lsp-rs-core::platform` module, then repoint perl-lsp-config to depend on perl-lsp-rs-core instead of perl-dap.

**Cycle problem:** 
```
perl-lsp-config → perl-dap::platform::resolve_perl_path_with_toolchain
  └─ creates: perl-lsp-config → perl-dap
  
But: perl-dap → perl-lsp-rs-core (at runtime, post-G3)
  └─ creates a 3-way coupling that makes perl-lsp-config hard to absorb
```

**Why this approach (Option A):**
- Platform functions are pure Perl detection logic (~60 LOC), no perl-dap-specific code
- Moving them to perl-lsp-rs-core makes them a stable, reusable service layer
- perl-lsp-rs-core is the natural home for LSP runtime utilities
- Breaks the cycle and allows config to be absorbed into rs-core cleanly
- perl-dap keeps platform functions from perl-lsp-rs-core re-export (if needed), eliminating direct config-dap dependency

**Why NOT Option B (duplicate config logic into perl-dap):**
- Would add ~150 LOC duplication to perl-dap
- Violates DRY; config logic would diverge over time
- Maintenance burden higher

**Why NOT Option C (invert cycle, perl-dap → perl-lsp-config):**
- Violates layering: perl-dap is LSP domain, shouldn't depend on config crate
- Adds spurious dependency to DAP

**Research fact:** perl-lsp-config/src/lib.rs line 8 currently imports `perl_dap::platform::resolve_perl_path_with_toolchain` — this is the only cross-crate dependency chain that matters. All other config code is self-contained.

---

### D6: perl-content-length-framing → perl-lsp-rs-core::transport::framing

**Decision:** Absorb into `perl-lsp-rs-core::transport::framing` (replacing re-export shim).

**Why:** Framing is a protocol-level utility (~150 LOC) used by both DAP and LSP tests. perl-lsp-rs-core already has a transport module and already depends on perl-dap at runtime (post-G3). Absorbing keeps transport logic co-located.

**Safety:** perl-dap already lists `perl-lsp-rs-core` as a runtime dependency (line 62 of perl-dap/Cargo.toml), so no new cycle introduced.

**Current state:** perl-lsp-rs-core/src/transport/framing.rs already exists and re-exports `perl_content_length_framing::*`. This PR replaces that re-export stub with actual module content.

**Rejects:**
- Duplicate into perl-dap — adds ~150 LOC maintenance burden, DAP shouldn't own transport protocols
- Keep published separately — conflicts with Amendment 6 trim targets
- Move to perl-dap — violates layering (transport is LSP/protocol concern, not DAP-specific)

---

### D1–D3: Parser satellites (ALREADY DONE, PR A merged)

See #4542 for context. PR A absorbed `perl-dead-code`, `perl-refactoring`, `perl-incremental-parsing` into `perl-parser`. Baseline was corrected from 37 → 34 at start of PR A. PR B starts from baseline = 34.

---

## Objections addressed

### From oppositional-planner

**Objection:** "Absorbing 3 unrelated crates into perl-lsp-rs-core violates single-responsibility principle."

**Resolution:** The 3 crates (feature-catalog, config, framing) are all **build-time or protocol-level utilities consumed by rs-core's own build.rs or transport layer**. They are not arbitrary domain logic. Absorbing keeps rs-core's build and transport infrastructure co-located. The alternative (keeping 3 standalone crates) adds noise to the published surface for features that are only used internally.

---

### From architecture-reviewer

**Objection:** "Moving 3 crates into one mega-crate creates maintenance risk."

**Resolution:** The three crates are being moved into **different modules within rs-core** (feature_catalog, config, transport::framing), not merged into a single god-module. Each retains logical separation. rs-core is already the largest and most-used crate in the workspace — it's the natural home for LSP infrastructure services. Post-Wave Final, the plan is to stabilize rs-core's module structure and document it as the foundation layer. This PR is part of that stabilization.

---

### From maintainer-issue

**Objection:** "Amendment 6 locked 'end at 31 published.' How does 34 → 31 fit the plan?"

**Resolution:** Plan-review corrected a baseline error: G3 left baseline.txt at 37, but actual post-G3 allowlist was 34. The orchestrator verified this against four independent sources and confirmed 34 is correct. PR A absorbed 3 crates (34 → 31), hitting the Amendment 6 target exactly. No tracker update needed.

---

## Research findings (plan-review verifications)

### Claim: perl-feature-catalog has no connection to perl-parser

**Verification:** Grep confirms:
- `perl-parser/Cargo.toml` has zero reference to perl-feature-catalog
- `perl-feature-catalog` has zero reference to perl-parser modules
- Only consumers are perl-dap/build.rs and perl-lsp-rs-core/build.rs (build-time only)
- **Status: CONFIRMED.** Prior ledger target was wrong.

### Claim: perl-lsp-config depends on perl-dap::platform, not elsewhere

**Verification:** 
- `perl-lsp-config/src/lib.rs` line 8: `use perl_dap::platform::resolve_perl_path_with_toolchain`
- No other perl-dap imports in config crate
- perl-dap does NOT import perl-lsp-config (no reverse cycle)
- **Status: CONFIRMED.** Single dependency chain.

### Claim: Three platform functions can be extracted cleanly from perl-dap

**Verification:**
- `resolve_perl_path_with_toolchain()` line 205–219: uses only std::path, std::env, std::process::Command
- `detect_perlbrew_perl()` line 221–235: uses only std::env, std::path
- `detect_plenv_perl()` line 237–256: uses only std::env, std::path
- No perl-dap-specific code, no callbacks, no trait dependencies
- **Status: CONFIRMED.** Safe to extract.

### Claim: perl-content-length-framing is protocol utility, not DAP-specific

**Verification:**
- Used by perl-dap (4 locations)
- Also used by perl-lsp tests (4 locations)
- Implements generic message framing (ContentLengthFramer struct, frame fn)
- Zero DAP-specific logic
- **Status: CONFIRMED.** Protocol utility, not DAP domain.

### Claim: G3 negative tests explicitly block absorption

**Verification:**
- `g3_config_stays_standalone.rs`: asserts `!std::path::Path::new("crates/perl-lsp-rs-core/src/config.rs").exists()` and checks ADR for "Wave H" label
- `g3_content_length_framing_stays.rs`: asserts framing stays published, no module in rs-core
- **Status: CONFIRMED.** Tests must be deleted before absorption works.

---

## Related issues

- **#4410** (parent tracker) — Master issue for Wave Final consolidation. Documents Amendment 6 target (31 published + 4 internal).
- **#4542** (PR A, merged 2026-04-21) — Wave 4-Completion. Absorbed parser satellites, corrected baseline 37 → 34. PR B depends on PR A being merged first.
- **#4405** (strict_subs overlap) — Separate PR affecting scope_analyzer.rs. Not directly related to this PR but should be triaged for merge order if both are in flight.
- **#4518** (red-TDD absorption API-read guard) — Process improvement for Red-TDD builders. Ensures API surfaces are documented before tests are written. Applies to this PR's red-TDD phase.

---

## Alternative paths considered and rejected

### Path 1: Keep 3 crates published but mark as "deprecated"

**Reject:** Doesn't meet Amendment 6 target (31 published). Deprecation without removal is a long-tail support burden.

### Path 2A: Absorb all 3 crates into perl-parser

**Reject:** perl-parser has zero dependency on feature-catalog or framing. Config doesn't belong in parser either (LSP runtime concern, not parsing). Would create false cohesion.

### Path 2B: Absorb all 3 crates into perl-dap

**Reject:** DAP is a debug protocol tool, not the LSP runtime. Config and framing are LSP-layer concerns. Violates layering.

### Path 2C: Create new "perl-lsp-infrastructure" crate as umbrella

**Reject:** Adds a new crate instead of consolidating. Contradicts Amendment 6 trim target.

### Path 3: Absorb config and feature-catalog, keep framing published

**Reject:** Framing is just as much a protocol utility as config. Leaves 35 published (vs. 31 target). Inconsistent.

---

## Wave and amendment context

- **Amendment 6** (2026-04-10): Locked target of 31 published crates + 4 internal. Enabled by planned Wave 4 and Wave Final absorptions.
- **Wave 4-Completion** (PR A, merged): Absorbed parser satellites (3 crates, 37 → 34).
- **Wave Final** (PR B, this issue): Absorb LSP deferrals (3 crates, 34 → 31). Closes Amendment 6.
- **Amendment 9** (to be added in this PR or follow-up): Documents Wave 4-Completion and Wave Final changes, including baseline correction (G3 left 37, actual 34) and ledger correction (feature-catalog target).

---

## Timeline

- **2026-04-10:** Amendment 6 locked target at 31 published
- **2026-04-17:** Plan-review complete for full Wave Final (both PR A + PR B)
- **2026-04-21:** Plan-review Correction 2 reversal — baseline confirmed at 34 (not stale 37)
- **2026-04-21:** PR A (Wave 4-Completion) merged, baseline = 34
- **2026-04-21:** PR B (this issue) spec-planner starts, baseline = 34, target = 31
- **TBD:** PR B spec-planner completes, branches for red-TDD → builder
- **TBD:** PR B merged, published count = 31, Amendment 6 target achieved

---

## Rollback and risk

**No breaking changes to public API:**
- The 3 absorbed crates are internal-only or build-time-only utilities
- `perl-lsp-rs-core::platform`, `perl-lsp-rs-core::config`, `perl-lsp-rs-core::transport::framing` are all new public modules (didn't exist before)
- Consumers simply switch imports from old crate names to new module paths
- No type signature changes or behavioral changes

**Rollback:** If a bug is found post-merge, the PR can be reverted cleanly (all changes are additive + import rewiring). No data migration or config version issues.

**Risk mitigation:**
- Red-TDD builder writes comprehensive tests before implementation
- Builder follows the strict change order in checklist (24 numbered steps)
- Each step is independently verifiable
- G3 tests updated to new baseline before this work starts
- layer-check run after every major change to catch cycles early

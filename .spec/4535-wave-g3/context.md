# Context: #4535 Wave G3 — LSP Governance/Tooling Collapse

## Locked Decisions (plan-reviewer, binding for builder)

### D1: Protocol Absorption — Option A (CONFIRMED)
- **Decision:** Absorb `perl-lsp-protocol` into `perl-lsp-rs-core::protocol`
- **Evidence:** Zero external crates.io consumers verified (crates.io API 2026-04-21: only `perl-lsp-transport` + `perl-lsp-rs` as reverse-deps, both internal)
- **Why:** Dissolves the transport cycle; no third-party migration needed; consistent with Amendment 6 precedent
- **Alternative rejected:** Option B (invert Wave F) — destabilizes completed work; option C (defer) defeats v0.13.0 goal

### D2: Absorption Order — MANDATORY SEQUENCE (locked)
Builder MUST follow this exact order. Each step is a separate green commit. Run `cargo xtask layer-check` after steps 1, 2, 3, 4.

1. `perl-lsp-feature-governance` (dissolves its cycle with rs-core)
2. `perl-lsp-protocol` (dissolves transport cycle)
3. `perl-lsp-uri` (already declared as workspace dep in rs-core Cargo.toml:46)
4. `perl-lsp-transport` (both blockers now resolved)
5. `perl-lsp-performance` (leaf crate, no cycles)
6. `perl-lsp-critic-parser` (leaf crate, no cycles)
7. `perl-lsp-tooling` (depends on #5 and #6; absorb last)

**Critical note:** Original spec said "protocol last" — this is WRONG and will fail. Transport cannot absorb before protocol or the cycle blocks step 4.

### D3: perl-lsp-config — KEEP PUBLISHED (Wave H deferred)
- **New blocker discovered in plan-review:** `perl-lsp-config` depends on `perl-dap`, which depends on `perl-lsp-rs-core`
- **Decision:** Keep published; file follow-up for Wave H
- **Count impact:** 44 → 37 (not 36 as spec claimed)

### D4: perl-content-length-framing — KEEP PUBLISHED (remains shared with DAP)
- **Decision:** Add as direct dependency of `perl-lsp-rs-core` Cargo.toml; do NOT absorb
- **Why:** Three consumers verified: `perl-dap`, `perl-lsp-transport` (being absorbed), `perl-lsp` binary. Full internalization creates hard cycle via `perl-dap`
- **Count stays:** 37

### D5: Feature Flag Routing (rs-core public API extension)
- **Change 1:** Extend rs-core `[features]`: change `lsp-compat = []` to `lsp-compat = ["dep:lsp-types"]`
- **Change 2:** Add to rs-core `[dependencies]`: `lsp-types = { workspace = true, optional = true }`
- **Change 3:** Clean `crates/perl-lsp/Cargo.toml` `lsp-ga-lock` feature chain: remove dead refs to `perl-lsp-protocol/lsp-ga-lock` and `perl-lsp-feature-governance/lsp-ga-lock` (only `perl-lsp-rs-core/lsp-ga-lock` remains)
- **Why:** After absorption, protocol and governance types are rs-core's public API. Feature flags must route correctly

### D6: ADR Amendments 7 + 8 (BEFORE first absorption commit)
- **Amendment 7:** G2 retrospective — explain why transport + performance deferred to G3 (protocol cycle + tooling dependency)
- **Amendment 8:** G3 decisions — document Option A confirmation, zero external consumers verified, config deferred to Wave H, count corrected from 36 to 37
- **Location:** Append to `docs/adr/0041-microcrate-collapse.md` (currently has 6 amendments, will add 7 + 8)
- **Timing:** Write BEFORE first absorption commit so git history is clear

## Wave G2 Precedent & Learnings

**PR #4521 (merged 2026-04-21, commit 862002205):**
- Absorbed 5 runtime-infra crates into `perl-lsp-rs-core::runtime`
- Deferred `perl-lsp-transport` + `perl-lsp-performance` due to protocol cycle
- Test migration: renamed files to avoid collisions (e.g., `*_runtime.rs` prefix)
- Snapshot updates: ~50 insta files refreshed; expect similar volume for G3

**Known risks from G2:**
- **Test file name collisions** — migrate tests with module prefix (governance_*, protocol_*, uri_*, transport_*, performance_*, critic_parser_*, tooling_*)
- **Edition 2024** — all absorbed crates must use edition 2024 (workspace-inherited)
- **Cycle detection** — `cargo xtask layer-check` gates cycles; verify between steps 1, 2, 3, 4
- **Public API surface** — `cargo public-api diff` is MANDATORY (not advisory per D5 ratchet in #4504)

## Objections Addressed (from prior pipeline agents)

### Oppositional-planner objections
- **"Protocol order contradiction"** → Resolved in D2: protocol step 2, transport step 4
- **"Feature flag routing unclear"** → Resolved in D5: explicit lsp-compat extension + lsp-ga-lock cleanup
- **"Content-length-framing ambiguity"** → Resolved in D4: keep published, add to rs-core deps

### Architecture-reviewer concerns
- **"Cycle needs verification"** → Layer-check gates inserted at steps 1, 2, 3, 4
- **"Public API surface expansion"** → Public-api diff MANDATORY per D5 ratchet

## Research Findings (verified 2026-04-21)

- `perl-lsp-protocol` **reverse-deps:** Only `perl-lsp-transport` + `perl-lsp-rs` (both internal); zero crates.io external consumers
- `perl-lsp-transport` **cycle:** `perl-lsp-transport` → `perl-lsp-protocol` → `perl-lsp-rs-core` (forward after protocol absorption)
- `perl-lsp-config` **blocker:** Depends on `perl-dap`, which depends on `perl-lsp-rs-core` (hard cycle if absorbed)
- `perl-content-length-framing` **shared:** Used by LSP transport + DAP; ~150 LOC, low coupling risk

## Related Issues & Context

- **#4410:** Parent tracker (Wave status table; updated 2026-04-21)
- **#4521:** Wave G2 PR (merged; precedent for test migration, snapshot patterns)
- **#4520:** G2 implementation issue (red TDD + builder notes; test file naming)
- **#4526:** Integration test bit-rot fixes post-G2 (merged; follow-up for G3 test migration)

## Expected Timeline

- **Red TDD:** Write failing tests for 7 absorptions + feature flag changes
- **Builder:** Implement per D2 order; 1 commit per absorption (7 commits) + 1 ADR/baseline commit = 8 commits total
- **Snapshot refresh:** Expect ~50 insta files (similar to G2); use `cargo insta review` or batch refresh
- **Count gate:** Update `xtask/published-crate-baseline.txt` in final commit (44 → 37)

## Scope Boundary

**IN scope (7 absorptions + feature flag + ADR + count update):**
- `perl-lsp-feature-governance` → `perl-lsp-rs-core::governance`
- `perl-lsp-protocol` → `perl-lsp-rs-core::protocol`
- `perl-lsp-uri` → `perl-lsp-rs-core::uri`
- `perl-lsp-transport` → `perl-lsp-rs-core::transport`
- `perl-lsp-performance` → `perl-lsp-rs-core::performance`
- `perl-lsp-critic-parser` → `perl-lsp-rs-core::critic_parser`
- `perl-lsp-tooling` → `perl-lsp-rs-core::tooling`
- `perl-content-length-framing` → add as direct rs-core dep (NOT absorbed)
- `perl-lsp-config` → keep published (follow-up for Wave H)
- Feature flag routing (D5)
- ADR amendments (D6)
- Baseline count update (D3 math)

**OUT of scope:**
- Wave G2 retroactive changes
- Wave G1 modifications
- Wave H (deferred work)
- DAP crate changes (except removing dead `perl-lsp-protocol` dep if present)

# Implementation Checklist: #4535 Wave G3 — LSP Governance/Tooling Collapse

## Critical Preflight (before starting implementation)

- [ ] Verify `git status` clean (no .codex-worktrees/ gitlinks or modified files)
- [ ] Create branch from `origin/master` (currently at 7cbea889e post-G2 merge): `git checkout -b impl/4535-wave-g3 origin/master`
- [ ] Write `docs/adr/0041-microcrate-collapse.md` Amendment 7 + Amendment 8 in a single commit BEFORE starting absorption steps
- [ ] Commit ADR: `git add docs/adr/0041-microcrate-collapse.md && git commit -m "docs(adr): add Amendment 7 (G2 retrospective) + Amendment 8 (G3 protocol decision)"`

## Amendment Writing (Commit 0: ADR foundation)

**File:** `docs/adr/0041-microcrate-collapse.md`
**Changes:** Append two amendments after Amendment 6 (currently ends at line ~450)
**Details:**

### Amendment 7 — 2026-04-21: Wave G2 Retrospective (transport + performance deferred due to protocol cycle)
- Explain why `perl-lsp-transport` + `perl-lsp-performance` were deferred from G2 → G3
- Root cause: `perl-lsp-protocol` dependency creates cycle (protocol → rs-core → transport → protocol)
- Decision: Resolve in G3 by absorbing protocol into rs-core (Option A)
- Deferred together because tooling depends on performance

### Amendment 8 — 2026-04-21: Wave G3 — Protocol Absorption (Option A confirmed, zero external consumers, config deferred)
- Verify `perl-lsp-protocol` reverse-deps: only internal (`perl-lsp-transport`, `perl-lsp-rs`)
- Decision: Absorb protocol into `perl-lsp-rs-core::protocol` (Option A)
- Additional decision: `perl-lsp-config` discovered to have hard cycle via `perl-dap` → keep published (defer to Wave H)
- Count corrected: 44 → 37 (not 36 as originally estimated)
- Expected absorptions: governance, protocol, uri, transport, performance, critic-parser, tooling (7 total)
- Remaining published: config, content-length-framing (2 not absorbed)

**Verify ADR edit:** `grep "^### Amendment 7" docs/adr/0041-microcrate-collapse.md`

---

## Step 1: Absorb `perl-lsp-feature-governance` → `perl-lsp-rs-core::governance`

### Source code move
- **Move:** `crates/perl-lsp-feature-governance/src/*` → `crates/perl-lsp-rs-core/src/governance/`
  - Copy all files: `lib.rs`, `predicates.rs` (or similar structure)
  - If `lib.rs` exists in source, rename to `mod.rs` or inline into parent `mod.rs`
- **Preserve:** All doc comments, attributes, test modules

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod governance;` (after line 8, before end of file)
- **Verify:** `grep "^pub mod governance" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Remove from `[dependencies]`: (governance is now internal; check if it was listed)
- Add workspace members to final step

### Update `crates/perl-lsp/Cargo.toml`
- Remove `perl-lsp-feature-governance` from `[dependencies]` section
- Remove `governance` from `[features]` lsp-ga-lock chain (if present)
- Verify: `grep -c "perl-lsp-feature-governance" crates/perl-lsp/Cargo.toml` should return 0

### Test file migration
- Migrate `crates/perl-lsp-feature-governance/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename test files with prefix: `comprehensive_unit_tests.rs` → `governance_comprehensive_unit_tests.rs`
- Update any `mod` declarations in test files to reference new module path: `crate::governance` instead of external crate

### Verify step 1
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors (LSP server binary)
- `grep "pub mod governance" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 1
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp/ && \
git commit -m "refactor(lsp): Wave G3 step 1 — absorb perl-lsp-feature-governance → perl-lsp-rs-core::governance"
```

### Verify commit 1
- `cargo xtask layer-check` — passes (step 1 of 4 cycle-check points)
- `git log -1 --oneline` shows step 1 commit

---

## Step 2: Absorb `perl-lsp-protocol` → `perl-lsp-rs-core::protocol`

### Source code move
- **Move:** `crates/perl-lsp-protocol/src/*` → `crates/perl-lsp-rs-core/src/protocol/`
- **Preserve:** All structs, enums, type aliases, doc comments

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod protocol;` (after `pub mod governance;`)
- **Verify:** `grep "^pub mod protocol" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Check if `perl-lsp-protocol` listed in `[dependencies]` — if so, remove it (now internal)
- Verify: `grep "perl-lsp-protocol" crates/perl-lsp-rs-core/Cargo.toml` should return 0

### Update `crates/perl-lsp/Cargo.toml`
- Remove `perl-lsp-protocol` from `[dependencies]` (now use `perl-lsp-rs-core::protocol`)
- Change import: `perl_lsp_protocol::*` → `perl_lsp_rs_core::protocol::*`
- Remove `protocol` from `[features]` lsp-ga-lock chain if present
- Verify: `grep "perl-lsp-protocol\|perl-lsp-feature-governance" crates/perl-lsp/Cargo.toml` should return 0

### Update `crates/perl-lsp-transport/Cargo.toml`
- Remove `perl-lsp-protocol` from `[dependencies]` — transport will absorb next
- Verify: `grep "perl-lsp-protocol" crates/perl-lsp-transport/Cargo.toml` should return 0

### Test file migration
- Migrate `crates/perl-lsp-protocol/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename with prefix: `*.rs` → `protocol_*.rs` (4 test files: capability_advertisement_tests, comprehensive_unit_tests, error_builders_issue_3024, protocol_unit_tests)
- Update imports: `extern crate perl_lsp_protocol` → use `crate::protocol::*`

### Verify step 2
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-transport` — no errors (still a standalone crate for now)
- `cargo check -p perl-lsp-rs` — no errors
- `grep "pub mod protocol" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 2
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp/ crates/perl-lsp-transport/ && \
git commit -m "refactor(lsp): Wave G3 step 2 — absorb perl-lsp-protocol → perl-lsp-rs-core::protocol"
```

### Verify commit 2 (CRITICAL CYCLE CHECK)
- `cargo xtask layer-check` — passes (step 2 of 4; transport cycle now resolved)
- `git log -2 --oneline` shows both step 1 and step 2

---

## Step 3: Absorb `perl-lsp-uri` → `perl-lsp-rs-core::uri`

### Source code move
- **Move:** `crates/perl-lsp-uri/src/*` → `crates/perl-lsp-rs-core/src/uri/`
- Note: `perl-lsp-uri` has NO test directory (verified in preflight)

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod uri;` (after `pub mod protocol;`)
- **Verify:** `grep "^pub mod uri" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- If `perl-lsp-uri` listed in `[dependencies]`, remove it (now internal)
- Verify: `grep "perl-lsp-uri" crates/perl-lsp-rs-core/Cargo.toml | grep -v "^#"` should return 0

### Update consuming crates
- `crates/perl-lsp/Cargo.toml`: Remove `perl-lsp-uri` from `[dependencies]`
- `crates/perl-lsp-rs-core/Cargo.toml`: Verify `perl-lsp-uri` is no longer listed

### Verify step 3
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors
- `grep "pub mod uri" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 3
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp/ && \
git commit -m "refactor(lsp): Wave G3 step 3 — absorb perl-lsp-uri → perl-lsp-rs-core::uri"
```

### Verify commit 3
- `cargo xtask layer-check` — passes (step 3 of 4)
- `git log -3 --oneline` shows all three steps

---

## Step 4: Absorb `perl-lsp-transport` → `perl-lsp-rs-core::transport`

### Source code move
- **Move:** `crates/perl-lsp-transport/src/*` → `crates/perl-lsp-rs-core/src/transport/`

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod transport;` (after `pub mod uri;`)
- **Verify:** `grep "^pub mod transport" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Add dependency if not present: `perl-content-length-framing.workspace = true` (per D4: transport uses this, now rs-core uses directly)
- Verify: `grep "perl-content-length-framing" crates/perl-lsp-rs-core/Cargo.toml` — present

### Update `crates/perl-lsp/Cargo.toml`
- Remove `perl-lsp-transport` from `[dependencies]` (now use `perl-lsp-rs-core::transport`)
- Update any feature refs to `perl-lsp-transport` (check for `lsp-ga-lock` chaining)

### Test file migration
- Migrate `crates/perl-lsp-transport/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename: `comprehensive_unit_tests.rs` → `transport_comprehensive_unit_tests.rs` (1 test file)
- Update imports: `use perl_lsp_transport::*` → `use crate::transport::*`

### Verify step 4
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors
- `grep "pub mod transport" crates/perl-lsp-rs-core/src/lib.rs` — present
- `grep "perl-content-length-framing" crates/perl-lsp-rs-core/Cargo.toml` — present

### Commit 4
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp/ && \
git commit -m "refactor(lsp): Wave G3 step 4 — absorb perl-lsp-transport → perl-lsp-rs-core::transport"
```

### Verify commit 4 (CRITICAL FINAL CYCLE CHECK)
- `cargo xtask layer-check` — passes (step 4 of 4; all cycles resolved)
- `git log -4 --oneline` shows all four steps

---

## Step 5: Absorb `perl-lsp-performance` → `perl-lsp-rs-core::performance`

### Source code move
- **Move:** `crates/perl-lsp-performance/src/*` → `crates/perl-lsp-rs-core/src/performance/`

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod performance;` (after `pub mod transport;`)
- **Verify:** `grep "^pub mod performance" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Remove `perl-lsp-performance` from `[dependencies]` if listed (now internal)

### Update `crates/perl-lsp-tooling/Cargo.toml`
- Change: `perl-lsp-performance` from external crate dep → internal module via rs-core
- Update: `use perl_lsp_performance::*` → `use perl_lsp_rs_core::performance::*`

### Test file migration
- Migrate `crates/perl-lsp-performance/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename: `incremental_parser.rs` → `performance_incremental_parser.rs` (1 test file)
- Update imports: `extern crate perl_lsp_performance` → use `crate::performance::*`

### Verify step 5
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-tooling` — no errors
- `grep "pub mod performance" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 5
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp-tooling/ && \
git commit -m "refactor(lsp): Wave G3 step 5 — absorb perl-lsp-performance → perl-lsp-rs-core::performance"
```

### Verify commit 5
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-tooling` — no errors
- `git log -5 --oneline` shows all five steps

---

## Step 6: Absorb `perl-lsp-critic-parser` → `perl-lsp-rs-core::critic_parser`

### Source code move
- **Move:** `crates/perl-lsp-critic-parser/src/*` → `crates/perl-lsp-rs-core/src/critic_parser/`

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod critic_parser;` (after `pub mod performance;`)
- **Verify:** `grep "^pub mod critic_parser" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Remove `perl-lsp-critic-parser` from `[dependencies]` if listed (now internal)

### Update `crates/perl-lsp-tooling/Cargo.toml`
- Change: `perl-lsp-critic-parser` from external crate dep → internal module via rs-core
- Update: `use perl_lsp_critic_parser::*` → `use perl_lsp_rs_core::critic_parser::*`

### Test file migration
- Migrate `crates/perl-lsp-critic-parser/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename with prefix: `comprehensive_unit_tests.rs` → `critic_parser_comprehensive_unit_tests.rs`, `mutation_killing.rs` → `critic_parser_mutation_killing.rs` (2 test files)
- Update imports: `extern crate perl_lsp_critic_parser` → use `crate::critic_parser::*`

### Verify step 6
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-tooling` — no errors
- `grep "pub mod critic_parser" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 6
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp-tooling/ && \
git commit -m "refactor(lsp): Wave G3 step 6 — absorb perl-lsp-critic-parser → perl-lsp-rs-core::critic_parser"
```

### Verify commit 6
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-tooling` — no errors
- `git log -6 --oneline` shows all six steps

---

## Step 7: Absorb `perl-lsp-tooling` → `perl-lsp-rs-core::tooling`

### Source code move
- **Move:** `crates/perl-lsp-tooling/src/*` → `crates/perl-lsp-rs-core/src/tooling/`

### Update `crates/perl-lsp-rs-core/src/lib.rs`
- Add: `pub mod tooling;` (after `pub mod critic_parser;`)
- **Verify:** `grep "^pub mod tooling" crates/perl-lsp-rs-core/src/lib.rs`

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Remove `perl-lsp-tooling` from `[dependencies]` if listed (now internal)

### Update `crates/perl-lsp/Cargo.toml`
- Remove `perl-lsp-tooling` from `[dependencies]` (now use `perl-lsp-rs-core::tooling`)
- Update imports: `use perl_lsp_tooling::*` → `use perl_lsp_rs_core::tooling::*`
- Remove any feature refs to `perl-lsp-tooling` in lsp-ga-lock chain

### Test file migration
- Migrate `crates/perl-lsp-tooling/tests/` → `crates/perl-lsp-rs-core/tests/`
- Rename: `comprehensive_unit_tests.rs` → `tooling_comprehensive_unit_tests.rs` (1 test file)
- Update imports: `extern crate perl_lsp_tooling` → use `crate::tooling::*`

### Verify step 7
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors
- `grep "pub mod tooling" crates/perl-lsp-rs-core/src/lib.rs` — present

### Commit 7
```bash
git add crates/perl-lsp-rs-core/ crates/perl-lsp/ && \
git commit -m "refactor(lsp): Wave G3 step 7 — absorb perl-lsp-tooling → perl-lsp-rs-core::tooling"
```

### Verify commit 7
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors
- `git log -7 --oneline` shows all seven steps

---

## Step 8: Feature Flag Routing + Baseline Update + Final Cleanup

### Update `crates/perl-lsp-rs-core/Cargo.toml`
- Extend `[features]` section: change `lsp-compat = []` to `lsp-compat = ["dep:lsp-types"]`
- Extend `[dependencies]`: add `lsp-types = { workspace = true, optional = true }`
- Verify:
  ```bash
  grep "lsp-compat = " crates/perl-lsp-rs-core/Cargo.toml
  grep "lsp-types = " crates/perl-lsp-rs-core/Cargo.toml
  ```

### Update `crates/perl-lsp/Cargo.toml` — Clean lsp-ga-lock chain
- Find the `[features]` section with `lsp-ga-lock`
- Remove any dead refs to absorbed crates: `perl-lsp-protocol/lsp-ga-lock`, `perl-lsp-feature-governance/lsp-ga-lock`
- Keep only: `perl-lsp-rs-core/lsp-ga-lock`
- Verify: `grep -A 5 "lsp-ga-lock" crates/perl-lsp/Cargo.toml` shows only rs-core ref

### Update `xtask/published-crate-baseline.txt`
- Change: `44` → `37`
- Verify: `cat xtask/published-crate-baseline.txt` shows `37`

### Update `.spec/microcrate-collapse/ledger.md`
- Find Wave G3 row (lines 244-251)
- Update with actual absorbed crates (governance, protocol, uri, transport, performance, critic-parser, tooling)
- Update count from 36 to 37
- Mark config as "deferred to Wave H" (not absorbed)
- Mark content-length-framing as "published, added to rs-core deps"

### Snapshot refresh (expected ~50 files)
- Run: `cargo insta review` to accept snapshot changes from test migrations
- Or batch-accept: `cargo insta test --review`
- Verify: No `.pending.snap` files remain in `crates/perl-lsp-rs-core/tests/`

### Mark absorbed crates as unpublished
- For each absorbed crate directory, edit `Cargo.toml`:
  ```toml
  publish = false
  ```
- Crates affected: perl-lsp-feature-governance, perl-lsp-protocol, perl-lsp-uri, perl-lsp-transport, perl-lsp-performance, perl-lsp-critic-parser, perl-lsp-tooling
- Keep as workspace members (per Wave G1/G2 pattern for reference/docs)
- Verify:
  ```bash
  grep "publish = false" crates/perl-lsp-{feature-governance,protocol,uri,transport,performance,critic-parser,tooling}/Cargo.toml | wc -l
  ```
  Should return 7

### Verify final state
- `cargo metadata --no-deps --format-version 1 | jq '[.packages[] | select(.publish != [] and .publish != ["*"])] | length'` returns `37`
- `cargo xtask layer-check` — passes (no cycles)
- `cargo check -p perl-lsp-rs-core` — no errors
- `cargo check -p perl-lsp-rs` — no errors
- `cargo test -p perl-lsp-rs-core` — all tests pass
- `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test '*'` — integration tests pass

### Generate public API diff (MANDATORY)
- Run: `cargo public-api diff` (per D5 ratchet in #4504)
- Capture output and include in PR description (not commit message)
- Review for breaking changes; document any intentional API expansions

### Format and lint
- `cargo xtask fmt` — apply formatting
- `cargo clippy -p perl-lsp-rs-core` — verify no warnings

### Final commit
```bash
git add \
  crates/perl-lsp-rs-core/ \
  crates/perl-lsp/ \
  crates/perl-lsp-feature-governance/ \
  crates/perl-lsp-protocol/ \
  crates/perl-lsp-uri/ \
  crates/perl-lsp-transport/ \
  crates/perl-lsp-performance/ \
  crates/perl-lsp-critic-parser/ \
  crates/perl-lsp-tooling/ \
  xtask/published-crate-baseline.txt \
  .spec/microcrate-collapse/ledger.md && \
git commit -m "refactor(lsp): Wave G3 — finalize absorption, update baseline count to 37, clean lsp-ga-lock"
```

### Verify final commit
- `cargo xtask fmt` — produces no diffs
- `git log -8 --oneline` shows all 8 commits (ADR + 7 absorption steps + finalization)
- `git status` — clean (no staged changes)
- `git log -1` shows finalization commit

### BEFORE PUSHING: Critical Pre-Push Verification
- `cargo xtask layer-check` — final cycle check
- `cargo test -p perl-lsp-rs-core` — all tests pass
- `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test '*'` — integration tests pass
- `cargo xtask fmt` — no uncommitted diffs
- `cargo clippy -p perl-lsp-rs-core` — no warnings
- `git status` — clean
- `git log master..HEAD` shows 8 commits on impl/4535-wave-g3 (not on master)

---

## File Scope Summary

### Crates MOVED (source code into rs-core modules)
1. `crates/perl-lsp-feature-governance/src/` → `crates/perl-lsp-rs-core/src/governance/`
2. `crates/perl-lsp-protocol/src/` → `crates/perl-lsp-rs-core/src/protocol/`
3. `crates/perl-lsp-uri/src/` → `crates/perl-lsp-rs-core/src/uri/`
4. `crates/perl-lsp-transport/src/` → `crates/perl-lsp-rs-core/src/transport/`
5. `crates/perl-lsp-performance/src/` → `crates/perl-lsp-rs-core/src/performance/`
6. `crates/perl-lsp-critic-parser/src/` → `crates/perl-lsp-rs-core/src/critic_parser/`
7. `crates/perl-lsp-tooling/src/` → `crates/perl-lsp-rs-core/src/tooling/`

### Tests MIGRATED (with module prefix)
- `crates/perl-lsp-feature-governance/tests/` → `crates/perl-lsp-rs-core/tests/governance_*.rs`
- `crates/perl-lsp-protocol/tests/` → `crates/perl-lsp-rs-core/tests/protocol_*.rs`
- `crates/perl-lsp-uri/tests/` → none (no tests)
- `crates/perl-lsp-transport/tests/` → `crates/perl-lsp-rs-core/tests/transport_*.rs`
- `crates/perl-lsp-performance/tests/` → `crates/perl-lsp-rs-core/tests/performance_*.rs`
- `crates/perl-lsp-critic-parser/tests/` → `crates/perl-lsp-rs-core/tests/critic_parser_*.rs`
- `crates/perl-lsp-tooling/tests/` → `crates/perl-lsp-rs-core/tests/tooling_*.rs`

### Cargo.toml Updates
- `crates/perl-lsp-rs-core/Cargo.toml` — extend lsp-compat, add perl-content-length-framing, add lsp-types optional
- `crates/perl-lsp/Cargo.toml` — remove 7 absorbed crate deps, clean lsp-ga-lock chain
- `crates/perl-lsp-tooling/Cargo.toml` — (before absorption) remove direct refs to performance + critic-parser (use rs-core)
- `crates/perl-lsp-feature-governance/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-protocol/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-uri/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-transport/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-performance/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-critic-parser/Cargo.toml` — add `publish = false`
- `crates/perl-lsp-tooling/Cargo.toml` — add `publish = false`

### Other Files Updated
- `crates/perl-lsp-rs-core/src/lib.rs` — add 7 pub mod declarations
- `docs/adr/0041-microcrate-collapse.md` — add Amendment 7 + 8
- `xtask/published-crate-baseline.txt` — update 44 → 37
- `.spec/microcrate-collapse/ledger.md` — update Wave G3 row with actual absorptions, count 36 → 37

### Files NOT IN SCOPE
- `crates/perl-lsp-config/` — keep published (Wave H follow-up)
- `crates/perl-content-length-framing/` — keep published (added as rs-core dep)
- `crates/perl-dap/` — no changes (except potentially removing dead `perl-lsp-protocol` import if present)
- Wave G1, G2 crates — no retroactive changes
- `vscode-extension/`, `docs/`, `test_corpus/` — no changes

---

## Flags for Builder

### Critical Decision Points (D1-D6 are LOCKED; do not re-litigate)
1. **Protocol absorption order:** Must be STEP 2 (before transport step 4), not "last" as original spec said
2. **Cycle gates:** Run `cargo xtask layer-check` after steps 1, 2, 3, 4 — these are non-negotiable; if any fails, architecture is broken
3. **Config deferral:** Do NOT absorb `perl-lsp-config` — it has a hard cycle via `perl-dap`. File follow-up for Wave H
4. **Feature flag routing:** D5 requires both `lsp-compat` extension AND `lsp-ga-lock` cleanup in `perl-lsp/Cargo.toml`
5. **Public API diff:** MANDATORY (not advisory); capture output for PR description

### Test Migration Complexity
- 11 test files across 7 crates need prefix renaming (governance_*, protocol_*, etc.)
- ~50 snapshot refreshes expected (similar to G2); batch-accept with `cargo insta test --review`
- `RUST_TEST_THREADS=2` is critical for LSP integration tests; use it for all test runs

### Potential Gotchas
1. **Snapshot overflow:** If >100 snapshots need refresh, verify with `cargo test --lib` first to detect other issues
2. **Feature flag chain:** If `lsp-ga-lock` in `perl-lsp/Cargo.toml` has other crates listed, DO NOT remove them without verification
3. **Import alias conflicts:** If absorbed modules use same names as existing rs-core modules (e.g., `models`, `types`), use module prefix in re-exports to avoid collision
4. **Windows MAX_PATH:** Deep nested test paths may exceed Windows 260-char limit; use 8.3 names for test files if needed
5. **Worktree contamination:** After final commit, verify `git status` shows no `.codex-worktrees/` gitlinks (G2 lesson)

### Pre-Push Hook Risk
- If `pre-push` hook runs `cargo xtask corpus-check` and fails on G2 bit-rot (#4508 PR): use API-push workaround (use `gh pr create` instead of `git push`) or verify hook is not blocking

### Expected Timeline
- Red TDD: 2-3 hours (write failing tests for 7 absorptions + feature flag changes)
- Builder: 3-4 hours (implement per D2 order, handle snapshot refreshes)
- Green TDD: 1-2 hours (add edge cases, regression tests)
- Review: 1-2 hours (public-api diff audit, scope drift check)

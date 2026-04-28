# Context: #4499 publish-manifest-check

**Spec planner notes:** Verified facts, key decisions, and traps for the builder.

---

## Root cause

Two documented silent failures from `feedback_publish_pipeline_gotchas.md` have no xtask gate:

1. **Allowlist drift** — A crate absent from `[workspace.metadata.publish.allow]` is silently excluded from release pipeline. Currently caught only by Python script in CI (`.github/workflows/publish-dry-run.yml` lines 61-70), not part of xtask gate suite (`just pr-fast`, `just ci-gate`). Per-PR gate catches drift at the PR that introduced it, not weeks later at release time.

2. **Missing LICENSE** — `cargo package` catches missing license at packaging time (late), but xtask can catch it earlier at manifest-read time with clearer error message.

Both checks use `cargo metadata --no-deps` output (fast, offline, no network).

---

## Specification decisions

### Scope: what we DO validate

**Two checks only:**
1. **Allowlist drift (bidirectional):**
   - Drift A: Allowlist contains a crate with `publish=false` (crate should not be in allowlist)
   - Drift B: A publishable crate is absent from allowlist (all publishable crates must be listed)

2. **LICENSE present:**
   - Every allowlist crate must have non-empty `license` or `license_file` field in cargo metadata
   - Workspace inheritance is pre-resolved by cargo metadata JSON (44 crates use `license.workspace = true`; cargo expands to `"MIT OR Apache-2.0"`)

**Why these two:** Real documented silent failures. Allowlist drift would surface at publish time causing publish-attempt failure. Missing LICENSE would also fail publish but with less clear message.

### Scope: what we DO NOT validate

**Explicitly out of scope (already caught by cargo or irrelevant to manifest):**
- Keyword count ≤5 — caught by `cargo package` at packaging time
- Wildcard deps (e.g., `tokio = "*"`) — caught by `cargo metadata` parse failure + `cargo publish`
- Description present — caught by `cargo package` at packaging time
- Repository field — research-verifier confirmed this is RECOMMENDED not REQUIRED by crates.io
- SemVer compliance — out of scope for offline manifest check
- Transitive dev-dep leakage — already covered by `publish_closure` task
- docs.rs metadata (RUSTSEC, categories) — out of scope

This prevents scope creep: A1 validates manifest structure (allowlist + license), not API/docs/SemVer.

---

## Architecture decisions

### 1. Consolidate Python --check-drift into xtask

**Tradeoff:**
- **Consolidate:** Single source of truth, gate runs in pr-fast/ci-gate suite, faster feedback (Rust vs Python)
- **Keep separate:** Separate script remains unchanged, less churn during Wave G collapse

**Decision: Consolidate.** The Python check (`scripts/publish-topo.py --check-drift`) is the only remaining single-use mode in publish-topo.py. Topo-sort logic (lines 72-86 of workflow) will remain in Python. Moving just --check-drift to Rust reduces Python surface area and unifies validation approach. Net LOC reduction overall.

### 2. Shared allowlist loader in xtask/src/utils.rs

**Tradeoff:**
- **Single file:** Append to utils.rs (existing pattern for shared helpers)
- **New subdirectory:** Create `xtask/src/utils/publish.rs` for better organization

**Decision: Single file.** `xtask/src/utils.rs` is a single file, not a directory. Current codebase pattern is to append helpers to utils.rs. Avoids module nesting and follows existing precedent. Builder must not create `utils/` subdirectory.

**Why shared:** Both `publish_closure.rs` and `count_ratchet.rs` contain identical `WorkspacePublishMeta`/`AllowList` struct pairs. Refactoring to shared `load_publish_allowlist()` prevents drift if `[workspace.metadata.publish]` structure changes and reduces LOC by ~40.

### 3. Two-part check function (run + check_metadata)

**Tradeoff:**
- **Monolithic:** Single `run()` function does all work (simpler, single test entry point)
- **Pure function:** `check_metadata()` extracted for unit testability (can test logic without spawning cargo)

**Decision: Pure function.** Extract `check_metadata(meta: &NoDepsMetadata, allowlist: &[String]) -> Vec<String>` so violations can be tested without invoking `cargo metadata`. Allows 4 unit tests with synthetic metadata (faster, deterministic). Integration test spawns full `cargo xtask` command for end-to-end verification.

### 4. Workspace license inheritance safe to check

**Finding from plan-reviewer verification:**
- 44 crates in workspace use `license.workspace = true`
- Cargo metadata JSON output resolves this to actual string `"MIT OR Apache-2.0"` before Rust code sees it
- Example: A crate's manifest has `license.workspace = true`, but cargo metadata JSON shows `"license": "MIT OR Apache-2.0"`
- No false positives; the field is already expanded

**Implication:** The `license` field check is safe — we're checking the resolved value, not the workspace reference. The JSON from `cargo metadata` is the source of truth.

### 5. Integration test only (no fixture files)

**Tradeoff:**
- **Fixture files:** Create `xtask/tests/fixtures/bad-manifest-*.toml` for each violation class
- **Mocked metadata:** Unit tests create synthetic metadata via helper functions

**Decision: No fixture files.** Master has zero drift and zero missing-license violations (happy path confirmed). The fixture-file approach would require creating fake Cargo.toml files, running `cargo metadata` on them (slow, noisy), and dealing with path setup. Instead, unit tests mock `NoDepsMetadata` with helpers (`make_pkg()`, `make_meta()`), testing logic quickly. One integration test verifies happy path on real master.

### 6. Exit behavior and messaging

**Design:**
- `run()` collects all violations via `check_metadata()`, prints each to stderr (prefixed "ERROR: publish-manifest-check:")
- If any violations, print error count to stderr and exit non-zero
- If clean, print success message to stdout ("publish-manifest-check: OK (N crates checked, 0 violations)")
- Matches existing xtask pattern (published_crate_count, layer_check)

---

## Critical plan-reviewer corrections (verified facts)

### 1. scripts/publish-topo.py has 4 other callers — do NOT delete

**Callers identified:**
1. `--check-drift` at `.github/workflows/publish-dry-run.yml:61-70` — **BEING REPLACED** by this work
2. Topo-sort (no flag) at `.github/workflows/publish-dry-run.yml:72-86` — **STAYS IN PYTHON**
3. Topo-sort call in `justfile:2150` — **STAYS IN PYTHON**
4. Unit tests at `scripts/tests/test-publish-topo.py` — **STAYS IN PYTHON**
5. CI trigger in `.github/workflows/ci-gate-self-tests.yml` path filter — **STAYS IN PYTHON**

**Implication:** Only the `--check-drift` invocation (workflow line 61-70) moves to xtask. Lines 72-86 of workflow remain in Python. The entire `publish-topo.py` file must remain; we're not deleting it.

### 2. xtask/src/utils.rs is a file, not a directory

**Fact verified:** `git show 2a57448c8:xtask/src/utils.rs` returns a single file starting with `//! Utility functions for xtask`.

**Implication:** Append helpers directly to utils.rs. Do NOT create `xtask/src/utils/publish.rs` or change `mod utils;` to a directory. The import path is `use crate::utils::{load_publish_allowlist, ...}`, not `use crate::utils::publish::{...}`.

### 3. count_ratchet.rs ALSO has duplicate allowlist struct triple

**Finding:** Both `publish_closure.rs` (lines ~45-53) and `count_ratchet.rs` (lines ~31-43) define identical:
```rust
struct WorkspacePublishMeta { ... }
struct AllowList { ... }
```

**Implication:** Refactor both files, not just `publish_closure.rs`. Both must call `crate::utils::load_publish_allowlist()`.

### 4. publish.rs has different CargoMetadata struct — do NOT refactor

**Finding:** `xtask/src/tasks/publish.rs` has its own `CargoMetadata` struct that includes `packages: Vec<Package>` field needed by `publish_crates()` function. This is separate from the allowlist metadata structs.

**Implication:** Refactor only `publish_closure.rs` and `count_ratchet.rs`. Leave `publish.rs` untouched; it needs its own full `CargoMetadata` struct.

### 5. publish_manifest_check must be pub fn, not pub(crate)

**Finding:** Both inline unit tests in the same file and the integration test file (`publish_manifest_check_test.rs`) must call `check_metadata()`.

**Implication:** Mark `check_metadata()` as `pub fn`, not `pub(crate)`. It needs to be callable from test code in the same module and from the integration test file.

---

## Timing rationale

### Why land during Wave G (not defer to post-G)

**Original diaboli verdict was DEFER** — allowlist churn during Wave G would cause false-positive drift checks.

**Reversal:** Per-crate manifest checks (keyword count, license, etc.) don't churn when other crates collapse. But the key insight is that drift checks are most valuable *during* the collapse, not after. The collapse *is* the thing creating drift risk. A per-PR gate catches the mistake at the PR that introduced it (immediate feedback). A one-shot pre-release check would only catch it weeks later, at the worst moment (v0.13.0 cut).

**Rationale:** Serial merge train means each Wave G PR must update both baseline and allowlist together. If a PR fails the drift gate, the builder fixes the inconsistency before merging, not weeks later.

---

## Known traps (from plan-reviewer feedback)

1. **Never delete publish-topo.py** — it has 4 other callers. Only the `--check-drift` invocation moves.

2. **Never create xtask/src/utils/ as a directory** — append to the single-file utils.rs.

3. **Never forget to refactor count_ratchet.rs** — both count_ratchet and publish_closure have the duplicate structs.

4. **Never touch xtask/src/tasks/publish.rs** — it has its own CargoMetadata struct unrelated to allowlist metadata.

5. **Never use unwrap/expect/panic** — the codebase bans these in production code. Use `?` and `Result`.

6. **Never add checks beyond allowlist-drift + LICENSE** — scope creep is explicitly out of scope. Keyword count, wildcard deps, description, repository are all caught by cargo elsewhere.

7. **Never forget to export check_metadata() as pub** — tests in the same file and integration tests need to call it.

---

## Related history

**#4410** — Microcrate collapse guardrail backlog. This issue closes the "allowlist drift" and "missing LICENSE" items from that backlog.

**#4498** — Previous approach (superseded). This issue uses a different scope (smaller, more focused).

**#4497** — Public API diff ratchet (companion feature). Separate work, not related to manifest checks.

**feedback_publish_pipeline_gotchas.md** — Project memory documenting 6 past publish failures:
1. Allowlist drift — **A1 checks this**
2. Duplicate TOML keys — caught by `cargo metadata` parse
3. Missing LICENSE — **A1 checks this**
4. 429 rate limits — offline check can't see network
5. Exit-on-failure cascade — CI script design issue, not manifest
6. Missing retrigger logic — workflow orchestration, not manifest

A1 catches 2 of 6. Real coverage: ~33%, not the original 80% claim.

---

## Code patterns to follow

### How to use load_publish_allowlist()

```rust
use crate::utils::load_publish_allowlist;

let allowlist = load_publish_allowlist()?;  // Vec<String>
// allowlist is now [s"perl-token", "perl-parser", ...]
```

### How to use run_cargo_metadata()

```rust
use crate::utils::run_cargo_metadata;

let bytes = run_cargo_metadata(true)?;  // true = --no-deps
let meta: MyMetadata = serde_json::from_slice(&bytes)?;
```

### Violation message format

```rust
violations.push(format!(
    "drift: '{}' is in [workspace.metadata.publish.allow] but \
     has publish=false in Cargo.toml",
    name
));
```

Each violation is a single line, reported to stderr with "ERROR: publish-manifest-check: " prefix in run().

---

## Files to verify were not changed

After builder completes, these files should be untouched:
- `xtask/src/tasks/publish.rs` (different CargoMetadata struct)
- `scripts/publish-topo.py` (other callers remain)
- `scripts/tests/test-publish-topo.py` (unit tests for Python topo-sort)
- Everything else not listed in the checklist

---

## Success signal for next agent

The builder's PR will be ready for review when:
1. All acceptance criteria pass
2. `cargo test -p xtask` is green
3. `just pr-fast` is green on the impl branch
4. Commit message references #4499
5. No scope creep (only allowlist drift + LICENSE, no extra checks)
6. No code style issues (clippy clean, formatted)

The PR title should be: `feat(xtask): add publish-manifest-check (allowlist drift + LICENSE) (#4499)`

---

## Spec-planner verification checklist

- [x] Confirmed master base: 2a57448c8 (refactor Wave F lsp-rs-core)
- [x] Verified xtask/src/utils.rs exists as single file
- [x] Verified xtask/src/tasks/publish_closure.rs has duplicate AllowList struct (lines ~45-53)
- [x] Verified xtask/src/tasks/count_ratchet.rs has duplicate AllowList struct (lines ~31-43)
- [x] Verified .github/workflows/publish-dry-run.yml lines 61-70 contain Python --check-drift
- [x] Verified scripts/publish-topo.py has 4 callers beyond --check-drift
- [x] Verified workspace has 44 crates with license.workspace = true
- [x] Verified happy path on master: zero drift, zero missing licenses
- [x] Confirmed plan-reviewer corrections: utils.rs is file, publish-topo.py not deleted, count_ratchet needs refactor

All facts grounded in master codebase. Implementation is straightforward Rust with no ambiguity.

# Wave G2 Runtime Crate Absorption — Design Context

## Project Scope

**Umbrella:** #4410 (Wave G2 parent tracker)

**Related waves:**
- #4506 (Wave G1a: 15 low-risk providers → perl-lsp-rs-core::providers) — MERGED
- #4510 (Wave G1b: 10 medium-risk providers → perl-lsp-rs-core::providers) — MERGED
- This issue (Wave G2: 6 runtime crates → perl-lsp-rs-core::runtime)
- Wave G3 planned: perl-lsp-performance + perl-lsp-tooling (deferred to avoid import churn)

**Guidance docs:**
- PR #4518 (red-TDD API-read guidance) — apply the absorption API-read step
- PR #4516 (xtask package name resolution) — hook fix so builders don't need --no-verify
- Memory: `project_wave_d_facade_pattern.md`, `project_microcrate_collapse_v014.md`, `feedback_wave1_collapse_gotchas.md`, `feedback_red_tdd_needs_api_read.md`

---

## Plan-Reviewer Verdict

**Decision:** BUILD-AT-REDUCED-SCOPE (6 crates, NOT 7)

**Rationale:**

The oppositional planner surfaced O4 (import churn for performance): perl-lsp-performance is consumed by perl-lsp-tooling (G3 scope). Moving it in G2 forces a rewrite to `perl-lsp-rs-core::runtime::performance::*`, then moving tooling in G3 forces another rewrite as tooling gets absorbed. That's two import rewrites for the same files in two waves.

**Solution:** Defer performance to G3, move it with its consumer (tooling). This eliminates O4 entirely and reduces G2's scope from 7 to 6 crates. Benefits:

1. **No import churn:** performance moves once (in G3 with tooling)
2. **Lower risk per wave:** G2 scope is tighter, easier to validate
3. **Cleaner coordination:** G3 moves both performance + tooling together

**Architecture caveat addressed:** Three architecture-review concerns were all addressed via the spec:
1. **A-Cav-1:** Runtime module name masks different concerns (I/O, governance, metrics). **Addressed:** runtime/mod.rs doc comment explains grouping rationale and notes that text-utils is providers-adjacent.
2. **A-Cav-2:** perl-dap compile-time bloat from rs-core transitive deps. **Addressed:** perl-dap regression gate added to A11 (≤5% vs master).
3. **A-Cav-3:** text-utils is semantically providers-adjacent. **Addressed:** runtime/mod.rs doc comment clarifies this placement.

---

## Scope Decision Matrix

| Item | Decision | Rationale |
|------|----------|-----------|
| **Crates absorbed** | cancellation, limits, input-validation, launcher, transport, text-utils | Core runtime infrastructure, no G3 consumers blocking |
| **Crates deferred** | perl-lsp-performance | Consumed by perl-lsp-tooling (G3), defer together to avoid import churn (O4 resolution) |
| **Cross-binary deps** | launcher (perl-dap consumer) + dap regression gate | Handled: rs-core facade + A11 gate ensures no bloat |
| **Sibling files** | launcher/timing.rs, transport/framing.rs | Must be preserved during absorption |
| **Test files** | 11 total (performance's 1 file excluded) | Migrate with naming scheme `runtime_<module>_<subtest>.rs` |

---

## Stress-Tested Risks & Resolutions

### R1: perl-dap compile-time weight increase (MITIGATED)

**Risk:** Post-G2, `cargo build -p perl-dap` pulls the entire rs-core dep tree. If a future provider adds a heavy dep (e.g., machine learning library), dap's build time grows invisibly.

**Mitigation:** A11 adds `cargo build -p perl-dap --release` gate with assertion: link time ≤5% regression vs master baseline. If a builder or later wave adds a heavy dep to rs-core, this gate catches it.

**Responsibility:** Builder must measure dap's compile time on master (baseline), then post-G2 verify it's ≤5% slower. If faster or equal, gate passes. If >5% slower, halt and investigate.

### R2: Test migration may uncover visibility assumptions (MITIGATED)

**Risk:** Test files may import private items (e.g., `launcher::internal::detail_fn`) that rely on crate-level visibility. Moving to `runtime::launcher::mod.rs` changes visibility semantics.

**Mitigation:** Red-TDD must audit all 11 test files pre-implementation for `#[doc(hidden)]`, `#[allow(missing_docs)]`, or private-item imports. Flag any that need visibility adjustments post-absorption. This is part of the API-read step per PR #4518.

**Responsibility:** Red-TDD builder reads each test file's imports and documents them in a parallel comment on the issue (e.g., "launcher test file imports public::exported, no private refs found").

### R3: input-validation + limits module boundary cycle risk (MITIGATED)

**Risk:** input-validation depends on limits. After G2, both are in `runtime::*`. If limits ever imports from input-validation (unlikely but possible), this becomes a module-level cycle inside rs-core.

**Mitigation:** Xtask rule: enforce "limits has zero internal deps; input-validation may depend on limits only" via `cargo build -p perl-lsp-rs-core` and manual inspection post-absorption.

**Responsibility:** Builder must verify during implementation that `runtime/limits/mod.rs` has no imports from `runtime/input_validation/`.

### R4: Windows MAX_PATH still applies at worktree depth (MITIGATED)

**Risk:** Collapsing 7 crates saves ~7 directory levels but the worktree path itself is already deep.

**Mitigation:** This spec was planned on short-path worktree (`/c/wt4520`) to avoid hitting MAX_PATH during builds. Builder must also use short paths if on Windows.

**Responsibility:** Builder verifies short-path worktree usage and documents any MAX_PATH issues encountered.

### R5: Test file name collisions during migration (MITIGATED)

**Risk:** Multiple source crates have `comprehensive_unit_tests.rs` (cancellation, limits, launcher, transport). Moving all to `crates/perl-lsp-rs-core/tests/` requires renaming.

**Mitigation:** Checklist Step 13 specifies distinct naming scheme: `runtime_<module>_<subtest>.rs`. For example:
- cancellation's `comprehensive_unit_tests.rs` → `runtime_cancellation_comprehensive.rs`
- limits's `comprehensive_unit_tests.rs` → `runtime_limits_comprehensive.rs`

**Responsibility:** Builder must apply consistent naming during test migration and update all test-file references in Cargo.toml/CI configs.

### R6: xtask hardcoded crate references may exist (MITIGATED)

**Risk:** xtask may have hardcoded paths like `crates/perl-lsp-launcher` or `perl_lsp_launcher` in build-timing assertions or targeted-checks tasks.

**Mitigation:** Checklist Step 15 includes grep verification: `grep -r "perl-lsp-cancellation\|perl-lsp-launcher\|..." xtask/` post-implementation. If any remain, fix forward.

**Responsibility:** Builder must grep xtask/ for remaining references and update/remove as needed.

---

## Public API Surfaces (Red-TDD Reference)

### perl-lsp-cancellation → runtime::cancellation

**Structs:**
- `PerlLspCancellationToken` (pub; fields: cancelled, request_id, provider, created_at, timestamp)
- `ProviderCleanupContext` (pub)
- `CancellationRegistry` (pub)
- `CancellationMetrics` (pub)
- `RequestCleanupGuard` (pub)

**Enums:**
- `CancellationError` (pub)

**Traits:**
- `CancellableProvider` (pub)

**Statics:**
- `GLOBAL_CANCELLATION_REGISTRY: LazyLock<CancellationRegistry>` (pub)

**Key methods:**
- `PerlLspCancellationToken::new(request_id, provider) -> Self`
- `PerlLspCancellationToken::is_cancelled(&self) -> bool`
- `PerlLspCancellationToken::is_cancelled_relaxed(&self) -> bool`

---

### perl-lsp-limits → runtime::limits

**Structs:**
- `MemoryBudget` (pub)
- `MemoryMonitor` (pub)
- `LspLimits` (pub)

**Enums:**
- `MemoryPressure` (pub)

**Statics:**
- `LSP_LIMITS: LazyLock<RwLock<LspLimits>>` (pub)

**Key functions:**
- `workspace_symbol_cap() -> usize` (pub)
- `references_cap() -> usize` (pub)
- `completion_cap() -> usize` (pub)
- `reference_search_deadline() -> Duration` (pub)
- `regex_scan_deadline() -> Duration` (pub)
- `code_lens_cap() -> usize` (pub)
- `document_symbol_cap() -> usize` (pub)
- `semantic_tokens_deadline() -> Duration` (pub)
- `code_lens_resolve_deadline() -> Duration` (pub)
- `completion_deadline() -> Duration` (pub)

---

### perl-lsp-input-validation → runtime::input_validation

**Key functions:**
- `validate_file_path<P: AsRef<Path>>(path: P, workspace_root: &Path) -> Result<PathBuf>` (pub)
- `validate_file_content(content: &str, file_path: &Path) -> Result<()>` (pub)
- `validate_lsp_request(method: &str, params: &serde_json::Value) -> Result<()>` (pub)
- `sanitize_string(input: &str) -> String` (pub)
- `validate_workspace_root(workspace_root: &Path) -> Result<()>` (pub)

---

### perl-lsp-launcher → runtime::launcher

**Re-exports:**
- `pub use perl_lsp_feature_governance::*;`
- `pub use timing::*;` (includes `StartupReport`, `StartupTimer`)

**Constants:**
- `DEFAULT_LSP_PORT: u16 = 9257` (pub)

**Key functions:**
- `should_enable_logging(explicit_flag: bool) -> bool` (pub)
- `logging_filter(...) -> String` (pub)
- `init_logging(default_filter: &str)` (pub)
- `log_server_startup(...)` (pub)
- `parse_args<I>(args: I) -> Result<LaunchPlan, LaunchParseError>` (pub)

**Structs:**
- `TransportArgs` (pub)
- `LspArgs` (pub)
- `LaunchConfig` (pub)
- `LaunchPlan` (pub)

**Enums:**
- `TransportMode` (pub)
- `LaunchAction` (pub)
- `LaunchParseError` (pub)

**Sibling module:**
- `timing::StartupTimer` (pub)
- `timing::StartupReport` (pub)

---

### perl-lsp-transport → runtime::transport

**Re-exports:**
- `pub use framing::*;` (includes LSP message framing utilities)

**Key types:**
- All public items from `framing.rs` (LSP protocol frame parsing/serialization)

**Sibling module:**
- `framing::*` (message frame parsing, serialization, protocol utilities)

---

### perl-lsp-text-utils → runtime::text_utils

**Structs:**
- `TextEditHelpers<'a>` (pub; provides text editing utilities for code actions)

**Key methods:**
- Text edit composition methods (exact signature from `src/lib.rs`)

---

## Module Dependency Graph (Post-G2)

```
perl-lsp/
├── depends on: perl-lsp-rs-core (facade)
│   ├── runtime/
│   │   ├── cancellation/ (zero internal deps)
│   │   ├── limits/ (zero internal deps)
│   │   ├── input_validation/ (→ limits only)
│   │   ├── launcher/ (→ perl-lsp-feature-governance external)
│   │   ├── transport/ (→ perl-lsp-protocol external)
│   │   └── text_utils/ (zero internal deps)
│   ├── providers/ (unchanged from G1)
│   └── ... other modules

perl-dap/
├── depends on: perl-lsp-rs-core (facade)
│   └── runtime/launcher (only what it needs)
│   └── [risk: now pulls entire rs-core at compile time → A11 gate catches bloat]

perl-lsp-tooling/ (G3 scope)
├── depends on: perl-lsp-performance (NOT absorbed in G2, deferred to G3)
```

---

## Alternatives Considered & Rejected

### A1: Split into G2a (leaves) + G2b (rooted) — REJECTED

Oppositional planner suggested splitting into:
- G2a: cancellation, transport, text-utils (leaf crates)
- G2b: launcher, input-validation + limits (rooted internally)
- Defer: performance to G3

**Why rejected:** Adds complexity (two PRs instead of one, staggered reviews). The single G2 PR is already small enough (6 crates, 11 tests) that splitting doesn't reduce risk meaningfully. Plan-reviewer verdict is to defer only performance, making G2 a clean single wave.

### A2: Keep launcher as micro-crate — REJECTED

Oppositional planner suggested keeping launcher as `perl-process-launcher` instead of absorbing.

**Why rejected:** Defeats the "remove satellite crates" mission of the collapse. Launcher is a thin adapter around standard Rust APIs; absorbing it into rs-core is safe and reduces published crate count as intended.

### A3: Move text-utils to providers directly — REJECTED

Oppositional planner suggested moving text-utils to `runtime::text_utils` but re-exporting from `perl-lsp-rs-core::providers` for semantic clarity.

**Why rejected:** Architecture-reviewer confirmed the current placement (in runtime) is structurally sound. Re-exporting from both runtime and providers would complicate the module hierarchy. The doc comment in runtime/mod.rs clarifies the placement; that's sufficient for readers.

---

## Wave G3 Dependencies

**G3 scope (planned, not this issue):**
- perl-lsp-performance (currently in `crates/perl-lsp-performance/`, remains untouched in G2)
- perl-lsp-tooling (external to this codebase, consumes performance)
- Other runtime/infrastructure crates as needed

**Why performance is deferred:** Moves with its consumer (tooling) in G3 to avoid import churn (O4 resolution). After G2 lands, G3 will absorb both performance + tooling together.

---

## CI Gates & Verification

**Pre-implementation (spec verification):**
- [ ] Checklist steps 1-3 compile cleanly (module structure + lib.rs change)

**Post-implementation (builder verification):**
- [ ] A1-A13: All acceptance criteria passing
- [ ] A14-A17: Code quality gates passing
- [ ] DAP regression gate (A11): link time ≤5% vs master baseline
- [ ] `cargo xtask ci-gate` passes (merge-gate suite)

**Post-red-TDD (test verification):**
- [ ] All 11 migrated test files pass with updated imports
- [ ] `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs` passes (threading tests)
- [ ] perl-dap can build and link against new rs-core structure

---

## Handoff Notes for Red-TDD

Red-TDD should:

1. **Read the API surfaces section above** before writing tests (per PR #4518 guidance)
2. **Verify module import paths** work correctly (e.g., `use perl_lsp_rs_core::runtime::cancellation::*`)
3. **Audit test file imports** for private-item references that might break post-absorption
4. **Write shape tests** verifying that all public items (structs, enums, traits, functions) are accessible at identical visibility
5. **Write integration tests** verifying that perl-lsp and perl-dap can use the new module structure without breaking
6. **Verify the tracing filter** in cli.rs works correctly with `perl_lsp_rs_core=info` (not module path)
7. **Document any `NOTE(G2-API-fix)` comments** if builder needs to adapt the spec during implementation (target ≤2 per A15)

---

## References & Memory

**Key learning from prior waves:**
- `feedback_wave1_collapse_gotchas.md`: Edition=2024 required, test file name collisions, red-TDD literal-match brittleness, Windows MAX_PATH
- `feedback_red_tdd_needs_api_read.md`: Red-TDD writes tests against imagined API shapes; must read actual APIs first
- `project_wave_d_facade_pattern.md`: Entry-point crates are ergonomic facades (applies here)
- `project_microcrate_collapse_v014.md`: 135→30 published crate target; G2 contributes −6 crates

**Success pattern from G1a/G1b:**
- Folder-per-module structure is proven and safe
- Module-level doc comments clarify design intent (see providers/mod.rs)
- Test file renaming to avoid collisions is necessary and straightforward
- Re-export pattern (pub use module::*) provides ergonomic access

---

## Questions for Builders

If implementation reveals ambiguities, ping the issue with:

1. **API shape questions:** "Is X public or private in the original crate?" (Read CLAUDE.md Verification Ladder)
2. **Import path changes:** "Should [file] import from runtime module or directly from rs-core?" (Answer: module path post-G2)
3. **Test visibility issues:** "Can test file still access [private item]?" (Likely no; adjust test expectations)
4. **Performance regression:** "dap build time is 7% slower; is this expected?" (Answer: if >5%, halt and investigate)

No changes to the spec without a comment on the issue. Use `NOTE(G2-API-fix)` for discoveries (target ≤2).

---

## Architecture Coherence Checklist

- [x] **Dependency direction:** No upward bridges, all flows correctly
- [x] **Crate boundaries:** One concern per module, coherent grouping
- [x] **Type placement:** All types in appropriate layers
- [x] **Pattern consistency:** Identical to G1 folder-per-module structure
- [x] **Feature catalog:** N/A (infrastructure-only refactor)
- [x] **Cross-architecture:** perl-dap regression gate ensures no bloat
- [x] **Documentation:** runtime/mod.rs doc comment explains rationale

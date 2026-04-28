# Wave G2 Runtime Crate Absorption — Acceptance Criteria

Extracted from plan-reviewer refined spec. Each criterion is testable and measurable.

## Core Acceptance Criteria

- [ ] **A1:** All 6 source trees (cancellation, limits, input-validation, launcher, transport, text-utils) are present in `perl_lsp_rs_core::runtime::<module>` with correct module structure (mod.rs + sibling files where applicable)

- [ ] **A2:** All 11 integration test files migrated to `crates/perl-lsp-rs-core/tests/runtime_*.rs` with updated import paths (old crate paths → new module paths)

- [ ] **A3:** All 6 workspace dependencies removed from `crates/perl-lsp/Cargo.toml` dependency section

- [ ] **A4:** All 6 workspace dependencies removed from workspace `Cargo.toml` `[workspace.dependencies]` section

- [ ] **A5:** All 5 import zones in `crates/perl-lsp/src/` updated to rs-core paths:
  - cancellation.rs: `pub use perl_lsp_rs_core::runtime::cancellation::*;`
  - cli.rs: `use perl_lsp_rs_core::runtime::launcher::*;` + tracing filter to `perl_lsp_rs_core=info`
  - security/validation.rs: `pub use perl_lsp_rs_core::runtime::input_validation::*;`
  - state/mod.rs: `pub use perl_lsp_rs_core::runtime::limits::*;`
  - transport/mod.rs: `pub use perl_lsp_rs_core::runtime::transport::*;`

- [ ] **A6:** `xtask/published-crate-baseline.txt` updated from 49 → 43

- [ ] **A7:** `cargo test --workspace --lib` passes (all unit tests)

- [ ] **A8:** `cargo test -p perl-lsp-rs-core --test 'runtime_*'` passes (all migrated integration tests)

- [ ] **A9:** `cargo clippy --workspace --lib` passes (no warnings)

- [ ] **A10:** `cargo xtask ci-gate` passes (full merge-gate suite)

- [ ] **A11:** perl-dap regression gate: `cargo build -p perl-dap --release` compile time ≤5% vs master baseline (no bloat from rs-core transitive deps)

- [ ] **A12:** Layer and crate count checks pass:
  - `cargo xtask layer-check` passes
  - `cargo xtask published-crate-count-check` returns 43

- [ ] **A13:** LSPT-threading tests pass: `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2` succeeds

## Scope Exclusions

- [ ] **Explicit:** `perl-lsp-performance` is NOT absorbed in G2 (remains in `crates/perl-lsp-performance/`, deferred to G3 with perl-lsp-tooling)

- [ ] **Explicit:** `crates/perl-lsp-performance/` directory still exists post-implementation

- [ ] **Explicit:** No changes to `features.toml` (LSP capability definitions remain unchanged)

## Code Quality Gates

- [ ] **A14:** Module-level doc comment present in `crates/perl-lsp-rs-core/src/runtime/mod.rs` explaining grouping rationale and noting that text-utils is providers-adjacent

- [ ] **A15:** ≤2 `NOTE(G2-API-fix)` comments in implementation code (red-TDD effectiveness target per #4513/#4518)

- [ ] **A16:** All sibling source files preserved during absorption:
  - `launcher/timing.rs` present in new location
  - `transport/framing.rs` present in new location

- [ ] **A17:** All public API surfaces (structs, enums, traits, functions) remain accessible at identical visibility levels post-absorption (verified via type-compatibility test)

---

## Test Strategy

**Red-TDD approach:** Write failing tests first that verify:
1. Module imports work correctly (e.g., `use perl_lsp_rs_core::runtime::cancellation::*` resolves)
2. Public items are re-exported (e.g., `PerlLspCancellationToken` is accessible)
3. Integrations work (e.g., perl-lsp can create cancellation tokens, perl-dap can create launcher configs)
4. No regressions in existing tests (all migrated tests pass with updated imports)

**Integration check:** 
- Verify perl-dap can depend on `perl-lsp-rs-core` without cycles
- Verify perl-lsp consumers (perl-lsp binary) work as before

---

## Success Metrics

- All 15 acceptance criteria pass
- Zero breaking changes to public APIs (all existing re-exports work)
- Zero compilation errors or warnings
- All CI gates green on impl branch
- DAP binary doesn't regress in link time (>5% is a failure)

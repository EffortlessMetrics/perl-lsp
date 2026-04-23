# Acceptance Criteria: #4535 Wave G3 — LSP Governance/Tooling Collapse

## Locked Acceptance Criteria (from plan-reviewer)

- [ ] Amendment 7 (G2 retrospective: transport+performance deferred due to protocol cycle) written to `docs/adr/0041-microcrate-collapse.md` before any absorption commit
- [ ] Amendment 8 (G3: Option A confirmed, zero external consumers, config deferred to Wave H, count corrected to 37) appended after Amendment 7
- [ ] `perl-lsp-feature-governance` absorbed into `perl-lsp-rs-core::governance` (Step 1)
- [ ] `perl-lsp-protocol` absorbed into `perl-lsp-rs-core::protocol` (Step 2)
- [ ] `cargo xtask layer-check` passes after Step 2 (no cycles introduced)
- [ ] `perl-lsp-uri` absorbed into `perl-lsp-rs-core::uri` (Step 3)
- [ ] `perl-lsp-transport` absorbed into `perl-lsp-rs-core::transport` (Step 4)
- [ ] `cargo xtask layer-check` passes after Step 4 (no cycles introduced)
- [ ] `perl-lsp-performance` absorbed into `perl-lsp-rs-core::performance` (Step 5)
- [ ] `perl-lsp-critic-parser` absorbed into `perl-lsp-rs-core::critic_parser` (Step 6)
- [ ] `perl-lsp-tooling` absorbed into `perl-lsp-rs-core::tooling` (Step 7)
- [ ] `perl-lsp-config` NOT absorbed — remain published; follow-up issue filed for Wave H
- [ ] `perl-content-length-framing` NOT absorbed — remain published; added as direct dependency of `perl-lsp-rs-core`
- [ ] `crates/perl-lsp-rs-core/Cargo.toml` extended: `lsp-compat = ["dep:lsp-types"]` and `lsp-types = { workspace = true, optional = true }` added
- [ ] `crates/perl-lsp-rs-core/Cargo.toml` includes `perl-content-length-framing` as direct dependency
- [ ] All 7 absorbed module pub re-exports declared in `crates/perl-lsp-rs-core/src/lib.rs`
- [ ] `crates/perl-lsp/Cargo.toml` cleaned: dead `perl-lsp-protocol/lsp-ga-lock` and `perl-lsp-feature-governance/lsp-ga-lock` feature refs removed (only `perl-lsp-rs-core/lsp-ga-lock` remains)
- [ ] Test files migrated with module prefix to avoid collisions (e.g., governance_*.rs, protocol_*.rs, uri_*.rs, transport_*.rs, performance_*.rs, critic_parser_*.rs, tooling_*.rs)
- [ ] All snapshot/insta files updated (expect ~50 refreshes; use `cargo insta review`)
- [ ] `xtask/published-crate-baseline.txt` updated: `44` → `37`
- [ ] `cargo metadata --no-deps --format-version 1 | jq '[.packages[] | select(.publish != [] and .publish != ["*"])] | length'` returns `37`
- [ ] `cargo xtask layer-check` passes (no dependency cycles)
- [ ] `RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test '*'` passes
- [ ] `cargo test -p perl-lsp-rs-core` passes
- [ ] `cargo public-api diff` output recorded in PR description (MANDATORY; per D5 ratchet #4504)
- [ ] `.spec/microcrate-collapse/ledger.md` Wave G3 row updated with actual absorbed crates and count corrected to 37
- [ ] All 7 absorbed crate Cargo.toml files have `publish = false` set (keep as workspace members per Wave G1/G2 pattern)
- [ ] `cargo xtask fmt` produces no diffs
- [ ] `cargo clippy -p perl-lsp-rs-core` produces no warnings
- [ ] No `.codex-worktrees/` gitlinks in `git status` (verify clean before pushing)

## Test Coverage Expectations

- **Snapshot refreshes:** G2 had ~50 insta updates; G3 will have similar volume (7 crates vs 5 in G2)
- **Integration tests:** All LSP integration tests must pass (test thread count critical for DAP interaction)
- **API stability:** `cargo public-api diff` must be reviewed before merge (feature-gating considerations per D5)
- **Cycle gates:** `layer-check` gates at steps 1, 2, 3, 4 are non-negotiable

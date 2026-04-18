# Specifications — work-aa31d3df

## Feature/Behavior Description

Security hardening and documentation updates arising from the 2026-04-11 end-of-session sweep (issue #4141). This is a change-conveyor work item that records decisions, fixes a broken CI gate, and creates a prioritized backlog for follow-up work.

## Sprint-Scale Changes

### 1. `deny.toml` — Add ignore entry for `RUSTSEC-2026-0097`

**File:** `deny.toml`

**Change:** Add to `[advisories].ignore` array:
```toml
{ id = "RUSTSEC-2026-0097", reason = "Rand unsoundness only triggered by custom loggers calling rand::rng(); perl-lsp does not install custom loggers. Revisit when rand is patched or usage pattern changes." }
```

**Acceptance criteria:**
- [ ] `cargo deny check advisories` exits with code 0 in CI
- [ ] The ignore entry includes a `reason` field explaining the exposure gap
- [ ] A `# TODO` or comment in `deny.toml` references the revisit trigger (rand patch)

---

## Backlog Items (Not Implemented in This Work Item)

### 2. `run_test_sub` Identifier Validation

**File:** `crates/perl-lsp/src/execute_command/provider.rs`

**Description:** Add a regex validation for `sub_name` to enforce Perl identifier shape: `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*$`. Reject non-matching values with a clear error returned to the LSP client.

**Acceptance criteria:**
- [ ] Valid Perl identifiers (e.g. `foo`, `My::Module::func`) are accepted without behavioral change
- [ ] Invalid inputs (e.g. `main::foo; die`, `` `id` ``, shell metacharacters) are rejected with an error message
- [ ] New unit tests cover at least: valid identifiers, injection attempts, empty string

**Non-goals:** This does not eliminate the `no strict 'refs'` + `&$sub()` pattern; it only enforces the identifier contract.

---

### 3. `validate_expression` Rename

**File:** `crates/perl-dap/src/security/mod.rs`

**Description:** Rename `validate_expression` → `reject_multiline_expression`. Update all call sites. Add a doc comment: "Only rejects newlines (`\n`) and carriage returns (`\r`); does not sanitize Perl side-effect constructs."

**Acceptance criteria:**
- [ ] `cargo build -p perl-dap && cargo test -p perl-dap` passes
- [ ] All call sites are updated to the new name
- [ ] A doc comment clarifies the function's narrow scope
- [ ] No broadening of the rejection set (no attempt to block backticks/system/eval)

**Non-goals:** No attempt to comprehensively sanitize Perl expressions. DAP debugger context already limits blast radius.

---

## Struck Items

### Finding 8 / SBOM Regeneration — REMOVED

**Finding 8 in the research report is factually incorrect.** The files `sbom-cyclonedx.json` and `sbom-spdx.json` **do not exist** in the repository. They are generated on-demand via `cargo sbom` during the release process. There is no "13-day drift." This task must not appear in any backlog, tracker, or subsequent report.

---

## Deferred Items (Needs Verification)

### Cargo Machete False Positive — `perllsp`

**File:** `crates/perllsp/Cargo.toml`

**Description:** The research report claimed `cargo machete` produces a false positive for `perl-lsp-rs` in the `perllsp` crate. The verification agent could not confirm this. The fix (`[package.metadata.cargo-machete] ignored = ["perl-lsp-rs"]`) should **not** be applied until:

1. `cargo machete` is run against the full workspace in CI
2. The false positive is confirmed with actual output
3. The root cause is understood (the `lib.rs` re-export may already be correct)

**Acceptance criteria (when verified):**
- [ ] `cargo machete` output shows the false positive for `perl-lsp-rs` in `perllsp`
- [ ] `[package.metadata.cargo-machete] ignored = ["perl-lsp-rs"]` is added to `crates/perllsp/Cargo.toml`
- [ ] CI confirms the false positive is silenced

---

## Dependencies

- `cargo deny` (via `nix develop -c just ci-gate` or `.github/workflows/ci-security.yml`)
- `cargo machete` — only if confirmed in CI

## Out of Scope

- Changes to `unsafe` block usage (already documented as properly annotated)
- Changes to banned construct enforcement (`perl-ci-hygiene` crate)
- The Windows extended-length path fix (#4089) — already reviewed and passing
- Runtime credential handling (already using env vars, no hardcoding)
- Any changes to the core parser, semantic analyzer, or LSP protocol implementation
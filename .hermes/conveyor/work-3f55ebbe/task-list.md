# Task List: Published Crate Count Ratchet Gate CI Integration

## Summary

The `published-crate-count` ratchet gate is implemented and wired into `justfile` (PR #4416). This work item adds the gate to `.ci/gate-policy.yaml` for formal CI integration.

## Tasks

- [ ] **Add `published_crate_count` entry to `.ci/gate-policy.yaml`**
  - Location: `merge_gate` tier section
  - Command: `cargo xtask published-crate-count`
  - Timeout: 30 seconds
  - Budget: 5 seconds
  - `quarantine: true` (gate is meaningful only post-collapse; current count 81 vs target ~30)
  - Tags: `ratchet`, `microcrate`, `collapse`

- [ ] **Verify gate runs correctly in CI environment**
  - Run `cargo xtask published-crate-count` locally and confirm expected behavior
  - Confirm gate appears in `cargo xtask gates --help` output

- [ ] **Verify baseline file is correct**
  - Confirm `xtask/published-crate-baseline.txt` contains `81` (current post-collapse count)

- [ ] **Document when to lift quarantine**
  - When collapse reaches ~30-31 crates (per ADR-0041)
  - Follow-up PR will change `quarantine: false` in gate-policy.yaml
  - Update baseline to final target value when that PR lands

## Notes

- The `count_ratchet.rs` implementation (167 lines) already exists and is fully functional
- The gate is wired into `just ci-published-crate-count` but not yet in formal gate-policy.yaml
- Current count: 81 crates (post Wave C/D collapses)
- Target: ~30-31 crates (per ADR-0041 as amended through Amendment 6)
- Baseline auto-tightens on each run where count has decreased since last run

## Verification Commands

```bash
# Verify gate runs and shows expected output
cargo xtask published-crate-count

# Verify baseline file
cat xtask/published-crate-baseline.txt

# Verify gate is in gates list
cargo xtask gates --help
```

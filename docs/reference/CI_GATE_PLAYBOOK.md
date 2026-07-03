# CI Gate Playbook — Rust Small + ripr+

> **Problem this addresses:** A correct, integration-tested production change
> can fail required proof gates or optional coverage telemetry in sequence and
> take 4+ iterations to land if you don't know the mechanics.
> Source: [#3089](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3089),
> observed on PR #3078.
>
> For how provider readiness changes interact with these gates, see
> [PROVIDER_READINESS_CONTRACT.md](PROVIDER_READINESS_CONTRACT.md).

---

## The required gates

Every PR must pass the required proof gates before merge:

| Gate | What it checks | Failure rate |
|---|---|---|
| **fmt + clippy** | Style and lint — run per-crate, not combined | Low once you know the local preflight |
| **Perl LSP Rust Small Result** | Routed Rust compile/test aggregate | Medium |
| **ripr+ New Gap Gate** | Mutation-killing tests at the new production call sites | Trips direct `--lib` unit tests |

`Codecov / Patch 95` is advisory and does not run on PRs or merge queues. Run
it on nightly/manual coverage lanes only; do not add tests solely to satisfy
changed-line coverage when the required proof gates are already green.

---

## Gate 1: fmt + clippy

### Local preflight

Run per-crate, separately. A combined `-p X -p Y` invocation glitches on some
versions:

```bash
cargo fmt --check -p perl-lsp-rs
cargo clippy -p perl-lsp-rs --lib -- -D warnings
```

Repeat for every crate your PR touches. `cargo xtask fmt` is available but takes
~33 minutes from cold (full xtask compile). For fmt-check only, the per-crate
`cargo fmt --check` is orders of magnitude faster.

### Common failure: master drift

If `main` has fmt or clippy violations when your PR rebases, CI fails on your PR
even though you didn't introduce the violation. Check `main` health first
(`git fetch && git diff main...HEAD -- '*.rs' | head -20`). If it's master drift,
a fmt-fix PR on `main` unblocks everyone.

---

## Advisory: Codecov / Patch 95

### What it measures

Codecov/Patch-95 measures line coverage of **changed lines only** in explicit
coverage runs, and only from **`--lib` tests** (i.e.,
`cargo test -p <crate> --lib`). Coverage from integration tests in `tests/`
does NOT count toward the patch metric.

**The gate-pincer trap:** you add a production change, cover it thoroughly with
an integration test in `tests/integration_test.rs`, CI shows the test passing —
and Codecov/Patch-95 still reports low advisory patch coverage because `--lib`
never executed those lines.

### Diagnosing the exact uncovered lines

1. Download the coverage-proof CI artifact:
   ```bash
   gh run download <codecov-run-id> -n coverage-proof
   ```
2. Read `receipts/quality/quality-gate-coverage.json` → look for
   `sample_uncovered_lines`.
3. In `lcov.info`, search for `DA:<line>,0` (line N, 0 hits) in the changed
   file sections.

### Fix

If the uncovered lines reveal a real behavior gap, add focused tests that prove
that behavior. Do not add tests solely to satisfy changed-line coverage; normal
PR merge proof comes from RIPR+ and focused Rust checks.

### Comment staleness

The Codecov PR comment updates only on new pushes. A green Codecov comment from
a previous push does not mean the current HEAD is green. Verify against the
current HEAD SHA's run.

---

## Gate 3: ripr+ New Gap Gate

### What it measures

ripr+ performs mutation analysis on changed production code and verifies that at
least one test kills each mutation (i.e., the test can distinguish correct from
mutated behavior). A `--lib` unit test that directly calls
`handle_references_inner(...)` proves the logic, but ripr+ wants evidence that
the *production call path* is exercised, not just the function in isolation.

**The gate-pincer in reverse:** the `--lib` tests you added for Codecov pass the
coverage gate but may fail ripr+ because they aren't production call-observation
tests.

### Diagnosing the exact failing seams

Download the ripr evidence from the CI run:

```bash
gh run download <ripr-run-id> -n ripr-evidence
cat ripr/review/comments.json
```

Each entry names the exact `file:line` seam that needs a mutation-killing test.

### Fix options

**Option 1 (preferred): production call-observation test.** Add a test that
drives the real production entry point (the JSON-RPC handler, not the internal
function). For LSP providers this means an integration test through the real
dispatcher. On Linux CI this is straightforward; on Windows the production path
may differ — extract the portable logic first.

**Option 2: discriminator tests.** Add tests that exercise BOTH branches of each
seam — one that confirms the branch takes the expected path, one that confirms
the other branch. ripr+ accepts this as equivalent mutation coverage.

**Option 3: documented suppression (last resort).** For seams that are genuinely
un-closable — ripr#1428/1429 let-chain / match-condition blind spots where ripr
0.9.x cannot mutation-trace even with full branch coverage — add a narrow,
documented, expiring entry to `policy/ripr-suppressions.toml`:

```toml
[[suppressions]]
file = "crates/perl-lsp-rs/src/providers/references.rs"
line = 42
reason = "let-chain seam: ripr#1428 cannot mutation-trace; covered by scenario_22 behavioral tests"
expiry = "2026-09-01"  # re-evaluate when ripr 1.0 lands
covering_tests = ["ux_scenario_22_references_cross_file_no_open_file_masking_hard_assert"]
```

Do NOT use Option 3 to suppress seams that are reachable and coverable. The
suppression must cite:
- the ripr issue that makes it un-closable
- the covering behavioral tests that verify the behavior independently
- an expiry date

---

## CI timing and rollup behavior

CX53 CI runs are slow: **~15-20 minutes typical**. A sparse rollup (some checks
pending, not all green) does NOT mean the run is stuck — it means CI is still
running. Wait for the full result before diagnosing a failure.

`UNSTABLE-with-required-green` (all required gates pass, some informative checks
pending or yellow) is **mergeable**. The informative checks are not required.
Precedent: PR #3094 merged under exactly this state.

---

## Practical checklist before pushing

```
[ ] cargo fmt --check -p <each-touched-crate>
[ ] cargo clippy -p <each-touched-crate> --lib -- -D warnings
[ ] cargo test -p <each-touched-crate> --lib     (fast focused proof; advisory coverage input)
[ ] cargo test -p <each-touched-crate> --test <integration-test>  (ripr call-observation)
[ ] Confirm: no test passes on un-fixed main (race guards must actually guard)
[ ] If ripr suppression added: cite the issue, covering tests, expiry date
```

---

## Related issues

| # | Description |
|---|-------------|
| [#3089](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3089) | Gate-pincer playbook (this doc's source) |
| [#3067](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3067) | ripr gate binary-only (stop compiling xtask on ripr runs; CX43 disk pressure fix) |
| ripr-swarm#1428/1429 | let-chain / match-condition seams un-closable in ripr 0.9.x |

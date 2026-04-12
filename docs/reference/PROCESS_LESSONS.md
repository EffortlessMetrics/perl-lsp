# Process Lessons

Engineering process rules distilled from incidents. Each entry documents a rule,
why it exists, and how to verify compliance.

For incident-specific post-mortems, see [`docs/project/LESSONS.md`](../project/LESSONS.md).

---

## CI Gate Self-Tests

**Rule: Every new CI gate must have a self-test.**

### Background

In April 2026, the publish dry-run gate (`publish-dry-run.yml`) was silently
false-failing on every Cargo.toml PR for hours. The gate ran and reported failure,
but the failure was in the gate's own infrastructure (Windows path handling in patch
config generation) — not in the crates being tested. No one noticed because there
was no test that verified the gate actually *catches real errors on valid infrastructure*.

The fix was to add a self-test that feeds known-bad inputs to the gate and asserts
non-zero exit, and a known-good input and asserts exit 0.

### Pattern

For every CI gate script, create a companion `scripts/tests/test-<gate-name>.sh` that:

1. **Clean fixture** — feeds a known-good input. Asserts exit 0.
   Proves the gate does not false-fail.

2. **Negative fixture(s)** — feeds one or more known-bad inputs. Asserts non-zero exit.
   Proves the gate actually fires on the class of error it claims to catch.

Assertions must be real, not hardcoded. The script must invoke the actual gate
(or its underlying mechanism) against real fixtures — not mock the result.

### Example: Publish Dry-Run Gate Self-Test

`scripts/tests/test-publish-dry-run-gate.sh` tests the publish packaging gate:

```
CASE 1: Clean minimal crate        → cargo package exits 0    (no false-fail)
CASE 2: Duplicate [package] key    → cargo metadata exits 101 (parse error caught)
CASE 3: Nonexistent dependency     → cargo package exits 101  (resolution error caught)
```

Run with: `bash scripts/tests/test-publish-dry-run-gate.sh`

### CI Integration

Add the self-test to `.github/workflows/ci-gate-self-tests.yml` under a paths filter
that includes the gate script and its self-test. This way the self-test runs whenever
either changes.

```yaml
on:
  pull_request:
    paths:
      - 'scripts/cargo-package-workspace-dry-run.sh'
      - 'scripts/tests/test-publish-dry-run-gate.sh'
```

### Gating New Gate PRs

When reviewing a PR that adds a new CI gate:

- Require a companion self-test in `scripts/tests/test-<gate-name>.sh`.
- Require the self-test is referenced in `ci-gate-self-tests.yml`.
- Require the self-test was actually executed (provide output in the PR description).

A gate without a self-test may silently false-fail (or false-pass) for extended periods
with no visibility.

### Anti-Patterns

- **Hardcoded pass**: A self-test that always exits 0 regardless of gate behavior
  is worse than no self-test — it provides false confidence.
- **Testing the wrong layer**: Self-tests must invoke the gate mechanism, not mock it.
  Testing that bash exits 0 from `echo "ok"` does not prove cargo catches bad TOML.
- **Missing the negative case**: Testing only the clean fixture proves the gate doesn't
  false-fail, but not that it catches errors. Always include at least one negative fixture.

---

## Ops Pre-Merge Guard

Hard-won lessons from the swarm pipeline. Each section is a pattern that caused real failures.

## §1 — Ops pre-merge guard

Before merging any PR, run the pre-merge check. It catches three failure modes that wasted CI cycles across multiple sessions:

```bash
just pre-merge-check <pr-number>
# or: bash scripts/pre-merge-check.sh <pr-number>
```

Fails non-zero (skip this PR) if any of:

- **Draft state**: `isDraft: true` — PR is still in review, not ready.
- **Missing label**: No `merge-ready` label — must pass through reviewer → `/pr-ready`.
- **Missing issue ref**: Title lacks `(#NNN)` — CI `validate-title` will block the merge anyway.

Source: draft/label race hit 5 PRs in the 2026-04-08 session (issues #3321, feedback_pr_draft_label_race.md, feedback_validate_title_issue_ref.md).

## §2 — Two-pass review is mandatory for non-docs PRs

Every non-docs PR needs both reviewer (haiku, standards) and reviewer-deep (sonnet, correctness). Two-pass review caught 4 real bugs at 12-16x ROI. Docs-only PRs are the exception: they may merge with `merge-ready` alone if the pre-merge guard classifies every changed file as docs-only.

## §3 — Merge in batches of 3

Merging faster than 3 causes CI cancellation cascade — rapid merges cancel each other's CI runs. Max 3 per batch.

## §4 — CARGO_TARGET_DIR isolation per worktree

Each agent worktree must set `CARGO_TARGET_DIR` to a per-branch path under `/tmp/`. Shared build artifact directories cause cross-contamination between concurrent agents.

## §5 — Draft/label race

`isDraft` and `merge-ready` are independent signals. A PR can have `merge-ready` while still in draft. The pre-merge guard (§1) catches this before CI is even invoked.

## §6 — validate-title CI check

CI enforces `(#NNN)` at the end of PR titles. If a PR title lacks this, the merge will fail at CI. The pre-merge guard (§1) catches this early.

---

## See Also

- [`LESSONS.md`](../project/LESSONS.md) — Incident post-mortems
- [`CI_LOCAL_VALIDATION.md`](../project/CI_LOCAL_VALIDATION.md) — Gate tiers and local validation
- `scripts/tests/` — All gate self-tests
- `.github/workflows/ci-gate-self-tests.yml` — CI workflow that runs self-tests

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

## See Also

- [`LESSONS.md`](../project/LESSONS.md) — Incident post-mortems
- [`CI_LOCAL_VALIDATION.md`](../project/CI_LOCAL_VALIDATION.md) — Gate tiers and local validation
- `scripts/tests/` — All gate self-tests
- `.github/workflows/ci-gate-self-tests.yml` — CI workflow that runs self-tests

# Review receipts

Review receipts are evidence artifacts. They capture what a reviewer observed and what they explicitly tried to falsify before issuing a verdict.

## Schema

Path: `.ci/receipts/schemas/review.schema.json`

Required fields:

- `kind` (must be `review`)
- `producer`
- `pr`
- `head_sha`
- `base_sha`
- `verdict` (`clean | needs_builder_fix | needs_diff_fix | needs_human | blocked_unknown`)
- `material_observations[]`
- `negative_checks[]`
- `blockers[]`
- `next_routes[]`
- `supersedes`

Optional helper field:

- `diff_classification` (`trivial | non_trivial`)

## Review doctrine encoded in receipt validation

1. **Clean on non-trivial diff requires material observations.**
   - A clean verdict is suspicious if it says “nothing to flag” while the diff is non-trivial.
   - For `diff_classification = non_trivial`, `material_observations` must contain at least one concrete finding.
2. **Clean verdict requires negative checks.**
   - Clean sign-off must record what was actively checked and not observed (for example, no label mutation side effects).
3. **Needs-fix verdicts cannot emit clean sign-off route intent.**
   - If verdict is `needs_builder_fix`, `needs_diff_fix`, `needs_human`, or `blocked_unknown`, `next_routes` must not include clean sign-off intent (`signoff:clean`, `signoff_clean`, `clean-signoff`).
4. **Receipts are evidence only.**
   - Review receipts must not mutate labels. They describe findings and suggested routes; label mutation belongs elsewhere.

## Validation fixtures

Fixtures used by tests live in:

- `xtask/tests/fixtures/review-receipts/clean-with-observations.json` (valid)
- `xtask/tests/fixtures/review-receipts/clean-without-observations.json` (invalid)
- `xtask/tests/fixtures/review-receipts/needs-builder-fix-with-clean-signoff-intent.json` (invalid)

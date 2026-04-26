# Review Receipts

Review receipts are **evidence artifacts**. They describe what the reviewer observed and
what they explicitly ruled out. They do **not** mutate labels, and they are not a state
builder implementation.

Schema: `.ci/receipts/schemas/review.schema.json`

## Required fields

Every review receipt carries:

- `kind` (`review`)
- `producer`
- `pr`
- `head_sha`
- `base_sha`
- `verdict` (`clean | needs_builder_fix | needs_diff_fix | needs_human | blocked_unknown`)
- `material_observations[]`
- `negative_checks[]`
- `blockers[]`
- `next_routes[]`
- `supersedes` (optional)

## Policy constraints

- Clean sign-off on a non-trivial diff must include **at least one** material observation.
  - Current validation treats clean receipts as requiring concrete observations.
- Clean sign-off must include **negative checks** (`negative_checks[]` cannot be empty).
- Needs-fix verdicts must **not** emit clean sign-off intent (`signoff_clean`) in `next_routes`.
- Receipts are evidence only; no label mutation is permitted in this contract.

## Why this exists

`"CLEAN, nothing to flag"` on a non-trivial diff is suspicious by doctrine. These receipts
force specific, auditable evidence so downstream automation can trust review conclusions.

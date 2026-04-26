# Review receipts

Review receipts are **evidence-only** artifacts used by CI/state routing to evaluate sign-off quality.
They do not mutate labels and they do not perform builder/state transitions directly.

Schema: [`.ci/receipts/schemas/review.schema.json`](../../.ci/receipts/schemas/review.schema.json)

## Required fields

Each receipt must include:

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
- `supersedes`

## Policy rules

- `clean` verdicts must include at least one `material_observations` entry.
- `clean` verdicts must include at least one `negative_checks` entry.
- `needs_builder_fix` and `needs_diff_fix` verdicts must **not** include `signoff_clean` in `next_routes`.
- Receipts are evidence records only; label mutation is intentionally out of scope.

## Example (clean)

```json
{
  "kind": "review",
  "producer": "codex",
  "pr": 6853,
  "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "base_sha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "verdict": "clean",
  "material_observations": [
    "Verified parser changes are mechanical and preserve token boundaries."
  ],
  "negative_checks": [
    "No panic!/unwrap!/expect! introduced in touched code.",
    "No mutation-side effects observed outside scoped crates."
  ],
  "blockers": [],
  "next_routes": ["signoff_clean"],
  "supersedes": null
}
```

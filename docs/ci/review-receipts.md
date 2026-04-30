# Review receipts: material observations required for clean sign-off

Review receipts are **evidence-only** JSON artifacts emitted by review agents. They document what was checked, what was not observed, and what route should happen next. They do not mutate labels and they do not implement state-building logic.

Schema: `.ci/receipts/schemas/review.schema.json`.

## Required fields

Every review receipt includes:

- `kind` (must be `review`)
- `producer`
- `pr`
- `head_sha`
- `base_sha`
- `verdict` (`clean` | `needs_builder_fix` | `needs_diff_fix` | `needs_human` | `blocked_unknown`)
- `material_observations[]`
- `negative_checks[]`
- `blockers[]`
- `next_routes[]`
- `supersedes`

## Doctrine encoded by the schema

1. **Clean sign-off requires material observations.**
   - A `clean` verdict must include at least one item in `material_observations`.
2. **Clean sign-off requires negative checks.**
   - A `clean` verdict must include at least one item in `negative_checks`.
3. **Needs-fix verdicts cannot emit clean sign-off intent.**
   - `needs_builder_fix`, `needs_diff_fix`, `needs_human`, and `blocked_unknown` receipts must not contain `signoff_clean` in `next_routes`.
4. **Receipts are evidence, not mutation instructions.**
   - The receipt states review evidence and suggested route only.
   - Label mutation is intentionally out of scope.

## Minimal examples

### Clean review

```json
{
  "kind": "review",
  "producer": "reviewer@ci",
  "pr": 6853,
  "head_sha": "1111111111111111111111111111111111111111",
  "base_sha": "2222222222222222222222222222222222222222",
  "verdict": "clean",
  "material_observations": [
    "Compared branch diff to base and observed only schema/docs/test additions for review receipts"
  ],
  "negative_checks": [
    "No label mutation code paths introduced"
  ],
  "blockers": [],
  "next_routes": ["signoff_clean"],
  "supersedes": null
}
```

### Needs builder fix

```json
{
  "kind": "review",
  "producer": "reviewer@ci",
  "pr": 6853,
  "head_sha": "3333333333333333333333333333333333333333",
  "base_sha": "4444444444444444444444444444444444444444",
  "verdict": "needs_builder_fix",
  "material_observations": [
    "State builder contract for review receipts is missing routing consumption"
  ],
  "negative_checks": [
    "No signoff label intent should be emitted while builder fix is required"
  ],
  "blockers": [
    "Builder does not parse review receipt verdict taxonomy yet"
  ],
  "next_routes": ["builder_fix"],
  "supersedes": null
}
```


## Label ownership

Reviewer agents emit structured review receipts/comments; they are not label authorities.
The queue reconciler owns applying/stripping review labels from **current-SHA** receipts and repairing contradictions.
Stale-SHA receipts are ignored.

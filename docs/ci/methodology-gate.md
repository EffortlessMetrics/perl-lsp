# Methodology Gate

The Methodology Gate catches contradictory PR methodology label states and writes a deterministic receipt.

## Scope

The gate is intentionally narrow:

- It does **not** mutate labels.
- It does **not** implement a full reconciler/state builder.
- It only detects impossible combinations and reports them.

## Policy Source

Contradiction rules are declared in:

- `.ci/policies/label-contradictions.toml`

Supported policy sections:

- `[[forbidden]]` for exact forbidden label combinations.
- `[[forbidden_pattern]]` for required-label + forbidden-glob combinations.

Example:

```toml
[[forbidden]]
labels = ["review-reviewed", "needs-builder-fix"]
reason = "sign-off and builder route are mutually exclusive"
```

## Command Usage

Fixture mode:

```bash
cargo xtask methodology-gate --fixture <json> --receipt target/receipts/methodology-gate.json
```

Live PR mode (GitHub CLI required):

```bash
cargo xtask methodology-gate --pr <number> --receipt target/receipts/methodology-gate.json
```

Optional flags:

- `--dry-run` — always succeeds and only emits findings.
- `--enforce` — contradictory states become command failure.
- `--format json` — print machine-readable output to stdout.

## Advisory Rollout

Current default behavior is advisory. Contradictions are surfaced in the receipt but do not fail unless `--enforce` is supplied.

## Merge Group Nuance

`merge_group` payloads may not expose reliable PR labels. In that case the workflow passes a fixture with `labels_available=false`; the receipt classification becomes `unknown` and the gate does not fail.

Label contradiction enforcement currently happens on `pull_request` runs, with `merge_group` kept informative until merge-ready receipt/state-builder plumbing exists.

## Closeout Hygiene Warning

As conservative guidance, the gate emits a warning when the PR body appears partial/scaffold/umbrella and still uses closeout keywords (`Closes/Fixes/Resolves`).

Partial implementations should prefer `Refs` or `Part of`.

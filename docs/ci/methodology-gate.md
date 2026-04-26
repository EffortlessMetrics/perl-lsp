# Methodology Gate

The Methodology Gate is a deterministic policy check for contradictory PR state labels.

## What it checks

Policy source: `.ci/policies/label-contradictions.toml`.

Current contradictions:

- `review-reviewed` + `needs-builder-fix`
- `diff-audited` + `needs-diff-fix`
- `maintainer-pr-reviewed` + `needs-maintainer-fix`
- `ci-green` + `needs-ci-fix`
- `deep-reviewed` + `needs-deep-review`
- `merge-ready` + any `needs-*`
- `auto-merge` + any `needs-*`

## Modes

- **Advisory (default):** contradictions produce warnings and a receipt classification of `warn`.
- **Enforce (`--enforce`):** contradictions fail the command and produce a receipt classification of `fail`.

## Commands

```bash
cargo xtask methodology-gate --fixture <json> --receipt target/receipts/methodology-gate.json
cargo xtask methodology-gate --pr <number> --receipt target/receipts/methodology-gate.json
cargo xtask methodology-gate --fixture <json> --receipt target/receipts/methodology-gate.json --enforce
```

Optional flags:

- `--dry-run` evaluates policy but skips writing the receipt file.
- `--format json` prints machine-readable output to stdout.

## Receipt semantics

The gate emits `target/receipts/methodology-gate.json` when requested.

Classification values:

- `pass`: no contradictions detected.
- `warn`: contradictions detected in advisory mode.
- `fail`: contradictions detected in enforce mode.
- `unknown`: lookup not available (for example, merge queue contexts where labels are unavailable).

For `merge_group` events, label lookup may be unavailable. In that case the workflow records
`classification = "unknown"` and defers strict label enforcement to `pull_request` runs.

## Closeout hygiene

The gate currently emits a conservative warning (not a failure) when PR body text combines:

- closing keywords (`Closes`, `Fixes`, `Resolves`) and
- partial/scaffold/umbrella language.

Use `Refs` or `Part of` for partial implementations.

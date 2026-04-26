# Methodology Gate

The Methodology Gate is a cheap deterministic policy check for contradictory PR label states.

## Purpose

This gate detects impossible combinations such as sign-off and fix-routing labels coexisting on the same pull request (for example, `review-reviewed` + `needs-builder-fix`). It does **not** mutate labels, assign labels, or reconcile state.

## Policy source

Rules are defined in:

- `.ci/policies/label-contradictions.toml`

Supported rule shapes:

- `[[forbidden]]` exact label sets
- `[[forbidden_pattern]]` one required label plus a forbidden glob (currently prefix-style like `needs-*`)

## Commands

Fixture mode:

```bash
cargo xtask methodology-gate --fixture <json> --receipt target/receipts/methodology-gate.json
```

PR mode:

```bash
cargo xtask methodology-gate --pr <number> --receipt target/receipts/methodology-gate.json
```

Optional flags:

- `--enforce`: treat contradictions as failures (non-zero exit)
- `--dry-run`: calculate output without writing receipt files
- `--format json`: print machine-readable output to stdout

Default mode is advisory (warnings).

## Merge queue nuance (`merge_group`)

When merge-queue payloads do not reliably expose PR labels, the gate emits `classification=unknown` and does not fail for label lookup unavailability. Label enforcement remains on `pull_request` runs until merge-ready receipts/state builder work lands.

## Closeout hygiene (conservative)

In advisory mode, the gate warns when a PR body appears partial/scaffold/umbrella while using closeout verbs (`Closes`, `Fixes`, `Resolves`).

Use `Refs` or `Part of` for partial implementations.

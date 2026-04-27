# Methodology Gate

The Methodology Gate is a deterministic policy check for contradictory pull request labels.
It is the first enforcement layer to catch impossible PR states before merge.

## Scope

Current scope is label contradiction detection plus a conservative closeout hygiene warning.
The gate **does not** mutate labels or attempt to reconcile state.

## Policy source

Rules are defined in:

- `.ci/policies/label-contradictions.toml`

Supported rule kinds:

```toml
[[forbidden]]
labels = ["review-reviewed", "needs-builder-fix"]
reason = "sign-off and builder route are mutually exclusive"

[[forbidden_pattern]]
required = "merge-ready"
forbidden_glob = "needs-*"
reason = "merge-ready cannot coexist with any active blocker"
```

## Command usage

```bash
cargo xtask methodology-gate --fixture <json> --receipt target/receipts/methodology-gate.json
cargo xtask methodology-gate --pr <number> --receipt target/receipts/methodology-gate.json
```

Flags:

- `--dry-run` skips writing receipt files.
- `--enforce` converts contradictions from advisory warnings into a failing exit code.
- `--format json` prints the gate result as JSON to stdout.

## Workflow mode

`.github/workflows/methodology-gate.yml` runs in **advisory mode** initially.
Contradictions are reported in receipts, but CI does not fail until enforcement is explicitly enabled.

## merge_group behavior

`merge_group` payloads may not reliably expose label state.
When labels are unavailable, the gate emits a receipt with `classification=unknown` and does not fail.
Label contradiction enforcement remains on `pull_request` events until merge-ready state receipts are available.

## Closeout hygiene warning

A conservative warning is emitted if PR body text looks like a partial/scaffold/umbrella implementation while using `Closes`, `Fixes`, or `Resolves` issue-close keywords.

Preferred wording for partial implementations:

- `Refs #<issue>`
- `Part of #<issue>`

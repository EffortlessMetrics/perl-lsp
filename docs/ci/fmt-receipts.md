# `cargo xtask fmt --check` receipts

`cargo xtask fmt --check` now runs every configured workspace formatting check before returning a non-zero status. The command writes a machine-readable receipt to `target/receipts/fmt.json` for both pass and fail outcomes.

## Why this exists

Formatting drift is often spread across multiple crates. Failing fast on the first crate forces iterative reruns and slows both local work and CI triage. The receipt captures the complete failure set in one pass so fix-forward tooling can apply deterministic remediations.

## Receipt location

- Runtime artifact: `target/receipts/fmt.json`
- Schema: `.ci/receipts/schemas/fmt.schema.json`

The runtime artifact is intentionally not committed.

## Receipt fields

Top-level fields:

- `check`: always `fmt`
- `schema_version`: receipt schema version
- `verdict`: `pass` or `fail`
- `classification`: always `fmt_drift`
- `failures[]`: one item per formatting failure
- `fix_forward_kind`: always `FMT_ONLY`
- `safe_auto_fix`: always `true`
- `repro.command`: exact reproducer command

Per failure fields:

- `tool`: formatting tool name (`rustfmt`)
- `crate`: workspace crate name
- `path`: drifted file path when available (falls back to crate manifest directory)
- `check_command`: exact check command that failed
- `fix_command`: exact command to apply formatting for that crate

## Typed fix-forward support

Consumers can treat this receipt as typed `FMT_ONLY` remediation input:

1. Read all entries from `failures[]`.
2. Execute each `fix_command`.
3. Re-run `repro.command` to verify the tree is clean.

This keeps formatting auto-fix bounded and safe (`safe_auto_fix: true`) without changing non-format behavior.

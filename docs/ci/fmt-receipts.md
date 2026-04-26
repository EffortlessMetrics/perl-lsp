# Fmt receipt contract

`cargo xtask fmt --check` now executes every configured workspace formatter check, aggregates every failure, and writes a machine-readable receipt to `target/receipts/fmt.json` before returning a non-zero exit code.

## Why this exists

Fmt drift can exist in multiple crates at once. Early exit behavior hides later failures and forces iterative reruns. The receipt records all failures in one pass, including exact check and fix commands, which enables typed fix-forward flows.

## Receipt location

- Runtime output: `target/receipts/fmt.json`
- Schema: `.ci/receipts/schemas/fmt.schema.json`

The runtime receipt is generated output and must not be committed.

## Receipt shape

Top-level fields:

- `check`: always `fmt`
- `schema_version`: semantic version string (currently `1.0.0`)
- `verdict`: `pass` or `fail`
- `classification`: always `fmt_drift`
- `failures[]`: one entry per failing crate/file/tool tuple
- `fix_forward_kind`: always `FMT_ONLY`
- `safe_auto_fix`: always `true`
- `repro.command`: exact command used to reproduce (`cargo xtask fmt --check`)

Failure item fields:

- `tool`: formatter tool name (`rustfmt`)
- `crate`: crate inferred from failing manifest path
- `path`: file path from rustfmt diff output (or manifest fallback)
- `check_command`: exact check command that failed
- `fix_command`: exact fix command to apply locally

## Operator usage

- Run `cargo xtask fmt --check` once.
- Inspect `target/receipts/fmt.json` for the complete failure set.
- Run each `fix_command` and rerun the check.

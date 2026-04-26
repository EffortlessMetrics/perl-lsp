# Fmt receipts

`cargo xtask fmt --check` now writes a structured receipt to:

- `target/receipts/fmt.json`

The command still runs every configured workspace formatting check and still exits non-zero when formatting drift exists. The difference is that it now waits until all checks complete, then emits one receipt containing every failure.

## Receipt contract

Schema: `.ci/receipts/schemas/fmt.schema.json`

Top-level fields:

- `check`: always `fmt`
- `schema_version`: receipt schema version
- `verdict`: `pass` or `fail`
- `classification`: always `fmt_drift`
- `failures[]`: one entry per failing package check
  - `tool`
  - `crate`
  - `path`
  - `check_command`
  - `fix_command`
- `fix_forward_kind`: always `FMT_ONLY`
- `safe_auto_fix`: always `true`
- `repro.command`: exact command to rerun the check

## Typed fix-forward support

`fix_forward_kind=FMT_ONLY` and `safe_auto_fix=true` let automation and operators route these failures to safe formatting-only fix-forward flows without guessing from raw logs.

## Example

```json
{
  "check": "fmt",
  "schema_version": "1.0.0",
  "verdict": "fail",
  "classification": "fmt_drift",
  "failures": [
    {
      "tool": "cargo fmt",
      "crate": "xtask",
      "path": "/repo/xtask/Cargo.toml",
      "check_command": "cargo fmt --manifest-path /repo/xtask/Cargo.toml -- --check",
      "fix_command": "cargo fmt --manifest-path /repo/xtask/Cargo.toml"
    }
  ],
  "fix_forward_kind": "FMT_ONLY",
  "safe_auto_fix": true,
  "repro": {
    "command": "cargo xtask fmt --check"
  }
}
```

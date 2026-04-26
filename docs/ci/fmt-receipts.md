# fmt receipts

`cargo xtask fmt --check` now writes a structured receipt at `target/receipts/fmt.json`.

## Behavior

- Runs every configured workspace formatting check target.
- Continues after individual `rustfmt` check failures to collect the full failure set.
- Exits non-zero only after collection and receipt emission complete.

## Receipt schema

Schema file: `.ci/receipts/schemas/fmt.schema.json`

Top-level fields:

- `check` (`"fmt"`)
- `schema_version`
- `verdict` (`"pass"` or `"fail"`)
- `classification` (`"fmt_drift"`)
- `failures[]`
  - `tool`
  - `crate`
  - `path`
  - `check_command`
  - `fix_command`
- `fix_forward_kind` (`"FMT_ONLY"`)
- `safe_auto_fix` (`true`)
- `repro.command` (`"cargo xtask fmt --check"`)

## Typed fix-forward

The receipt is designed for typed fix-forward tooling:

- `classification=fmt_drift`
- `fix_forward_kind=FMT_ONLY`
- exact `fix_command` per failure entry

This keeps check mode read-only while making auto-remediation planning deterministic.

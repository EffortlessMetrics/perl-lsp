# Generated File Ownership Policy

Generated status/docs outputs are declared in `.ci/generated-files.toml` and are protected by ownership checks.

## Commands

- `cargo xtask generated-files list`
- `cargo xtask generated-files check --receipt target/receipts/generated-files.json`

## Behavior

- Detects changed files that match protected generated patterns.
- Requires a matching generator receipt (`owner` + `command`) unless `--allow-missing-receipt` is explicitly used.
- Emits a receipt with: `verdict`, `changed_files`, `expected_command`, and `missing_receipts`.
- Does **not** run generators automatically.

## Scope

Current protected scope:

- `docs/project/status/**` (owner: `status-docs`, command: `cargo xtask status-docs`)

This policy intentionally targets generated outputs and does not block hand-authored forensic documentation.

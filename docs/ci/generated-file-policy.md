# Generated File Ownership Policy

`cargo xtask generated-files` enforces ownership for generated paths declared in `.ci/generated-files.toml`.

## Why this exists

Generated status/docs files can drift when hand-edited without the corresponding generator run receipt. This command keeps ownership checks narrow to declared generated paths and avoids blocking hand-authored forensics docs.

## Commands

- `cargo xtask generated-files list`
- `cargo xtask generated-files check --receipt target/receipts/generated-files.json`

## Check behavior

- Detects changed files from git by default.
- Supports `--fixture <path>` for deterministic tests.
- Fails when changed generated files are missing a matching generator receipt, unless `--allow-manual-edits` is provided.
- Writes a receipt with `verdict`, `changed_files`, `expected_command`, and `missing_receipts`.
- Does not run generators automatically.

## Manifest format

```toml
[[generated]]
path = "docs/project/status/**"
command = "cargo xtask update-status --write"
owner = "status-docs"
allow_manual_edits = false
```

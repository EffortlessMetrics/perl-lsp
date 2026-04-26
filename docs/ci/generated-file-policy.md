# Generated-file ownership policy

`cargo xtask generated-files` enforces ownership for generated files declared in `.ci/generated-files.toml`.

## Commands

- `cargo xtask generated-files list`
- `cargo xtask generated-files check --receipt target/receipts/generated-files.json`

`check` inspects changed files, matches them against configured generated-file globs, and fails when ownership receipts are missing.

## Receipt shape

`check` writes a JSON receipt with:

- `verdict`
- `changed_files`
- `expected_command`
- `missing_receipts`

The command does not run generators automatically. Use explicit generator commands and rerun the check.

## Test fixtures

Integration tests can use `--fixture <path>` to provide deterministic changed-files and receipt-owner inputs.

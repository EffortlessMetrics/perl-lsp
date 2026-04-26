# Generated file ownership policy

Generated docs/status files are owned by explicit generator commands declared in `.ci/generated-files.toml`.

## Commands

- Check ownership: `cargo xtask generated-files check --receipt target/receipts/generated-files.json`
- List ownership rules: `cargo xtask generated-files list`

## Enforcement behavior

- The check detects changed files that match generated ownership rules.
- If `allow_manual_edits = false`, changes require matching generator receipt evidence or `--allow-override`.
- The command writes an ownership receipt with:
  - `verdict`
  - `changed_files`
  - `expected_command`
  - `missing_receipts`

## Scope guardrails

- Only paths declared in `.ci/generated-files.toml` are treated as generated ownership paths.
- Hand-authored forensics documents remain unaffected unless explicitly listed in the manifest.
- The checker does not run generators automatically.

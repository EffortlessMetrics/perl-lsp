# Typed fix-forward classification

`cargo xtask fix-forward` classifies CI receipts into explicit repair playbooks instead of routing all failures to a generic retry lane.

## Commands

- `cargo xtask fix-forward classify --receipt <receipt.json> --output target/receipts/fix-forward.json`
- `cargo xtask fix-forward list-playbooks`

## Output contract

The classifier writes a receipt with:

- `classification`
- `fix_forward_kind`
- `safe_auto_fix`
- `command`
- `route`
- `evidence`
- `next_agent`

Schema: `.ci/receipts/schemas/fix-forward.schema.json`.

## Initial kinds

- `FMT_ONLY`
- `TITLE_FIX`
- `STALE_BASE_CASCADE`
- `GENERATED_DOC_REGEN`
- `INFRA_ADVISORY_DEMOTION`
- `PARSER_RATCHET_REGRESSION`

Playbook source of truth: `.ci/fix-forward/playbooks.toml`.

## Notes

Current implementation only classifies and emits receipts. It does **not** mutate branches, create PRs, or apply labels.

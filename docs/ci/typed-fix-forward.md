# Typed fix-forward classification

`cargo xtask fix-forward` maps failing receipts to typed repair lanes so follow-up can route to a specific playbook instead of generic retries.

## Commands

- `cargo xtask fix-forward classify --receipt <receipt.json> --output target/receipts/fix-forward.json`
- `cargo xtask fix-forward list-playbooks`

## Initial playbooks

Playbooks are defined in `.ci/fix-forward/playbooks.toml`.

- `FMT_ONLY` — safe auto-fix using `cargo xtask fmt`
- `TITLE_FIX` — safe title mutation only
- `STALE_BASE_CASCADE` — route to `cascade-update`
- `GENERATED_DOC_REGEN` — docs regeneration lane (`cargo xtask status-docs`), manual for now
- `INFRA_ADVISORY_DEMOTION` — route to infra
- `PARSER_RATCHET_REGRESSION` — route to parser-builder

## Receipt fields

The classifier writes:

- `classification`
- `fix_forward_kind`
- `safe_auto_fix`
- `command`
- `route`
- `evidence`
- `next_agent`

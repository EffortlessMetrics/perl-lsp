# Typed fix-forward classification and playbooks

This document introduces typed fix-forward receipts for CI failures so retries can route to precise repair lanes.

## Commands

```bash
cargo xtask fix-forward classify --receipt <receipt.json> --output target/receipts/fix-forward.json
cargo xtask fix-forward list-playbooks
```

## Output receipt fields

The classifier emits the following fields:

- `classification`
- `fix_forward_kind`
- `safe_auto_fix`
- `command`
- `route`
- `evidence`
- `next_agent`

Schema: `.ci/receipts/schemas/fix-forward.schema.json`.

## Initial playbooks

Configured in `.ci/fix-forward/playbooks.toml`:

- `FMT_ONLY` → safe auto-fix via `cargo xtask fmt`
- `TITLE_FIX` → title-only mutation lane
- `STALE_BASE_CASCADE` → non-auto-fix cascade update lane
- `GENERATED_DOC_REGEN` → generated docs regen lane (manual today)
- `INFRA_ADVISORY_DEMOTION` → infra advisory lane
- `PARSER_RATCHET_REGRESSION` → parser builder lane

## Current scope

- Classifies existing CI receipts into typed fix-forward kinds.
- Lists configured playbooks for operational visibility.
- Does **not** mutate branches, open PRs, or apply labels yet.

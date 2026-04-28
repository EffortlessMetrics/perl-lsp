# Typed fix-forward playbooks

`cargo xtask fix-forward` classifies a failing CI receipt into a typed repair lane.

## Commands

```bash
cargo xtask fix-forward classify --receipt <receipt.json> --output target/receipts/fix-forward.json
cargo xtask fix-forward list-playbooks
```

## Receipt output

The classifier writes:

- `classification`
- `fix_forward_kind`
- `safe_auto_fix`
- `command`
- `route`
- `evidence`
- `next_agent`

The output schema lives at `.ci/receipts/schemas/fix-forward.schema.json`.

## Initial playbooks

- `FMT_ONLY` → safe auto-fix (`cargo xtask fmt`)
- `TITLE_FIX` → safe title-only mutation lane
- `STALE_BASE_CASCADE` → cascade update route
- `GENERATED_DOC_REGEN` → generated docs regeneration lane
- `INFRA_ADVISORY_DEMOTION` → infra route
- `PARSER_RATCHET_REGRESSION` → parser-builder route

## Current scope

This phase only classifies and emits a receipt. It does **not** mutate branches,
open PRs, or label PRs.

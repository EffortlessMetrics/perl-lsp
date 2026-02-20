# LSP Missing Features & Test Coverage Report

> **Last Updated**: 2026-02-17
> **Source of truth**: `features.toml` (capabilities) + `docs/CURRENT_STATUS.md` (metrics)
>
> This report lists **non-advertised** or **planned/preview** features only.
> If this file conflicts with `features.toml`, update this file.

---

## Summary

- **Advertised LSP features** are tracked in `features.toml` and should be stable and tested.
- **Missing/Not advertised LSP features** are limited to notebook support (preview).
- For **coverage metrics**, see `docs/CURRENT_STATUS.md` (do not restate numbers here).

---

## Missing / Not Advertised LSP Features (from `features.toml`)

### Notebook Support (Preview)

1. **`lsp.notebook_document_sync`** - Notebook document synchronization
   - Status: preview, not advertised
   - Implemented: handlers + capability gating
   - Tests: `tests/lsp_comprehensive_3_17_test.rs`

2. **`lsp.notebook_cell_execution`** - Execution summary tracking
   - Status: preview, not advertised
   - Implemented: executionSummary tracking
   - Tests: `tests/lsp_comprehensive_3_17_test.rs`

---

## Related DAP Gaps (Tracked in `features.toml`)

These are DAP items but often requested alongside LSP functionality.

- **`dap.breakpoints.hit_condition`** - preview, not advertised
- **`dap.breakpoints.logpoints`** - preview, not advertised
- **`dap.exceptions.die`** - preview, not advertised

Advertised DAP features:

- **`dap.core`** - core launch/break/step/inspect/evaluate/setVariable loop
- **`dap.breakpoints.basic`** - verified/unverified breakpoint lifecycle
- **`dap.inline_values`** - custom inlineValues request support

For DAP roadmap items (attach, variables/evaluate, safe eval), see `docs/ROADMAP.md`.

---

## Test Coverage Notes

- Per-feature tests are declared in `features.toml`.
- Missing/preview features above are covered by the tests referenced in `features.toml`.

---

## How to Update This Report

1. Update `features.toml` (capability status is the source of truth).
2. Run `just status-update` to refresh computed docs.
3. Re-align this file if any items changed.

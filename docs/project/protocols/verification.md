# Verification Protocol

> Process policy — not current state. Edit when process changes, not when metrics change.

## Tier A: Merge Gate (required for all merges)

```bash
just ci-gate  # ~2-5 min
```

## Tier B: Release Confidence (large changes or release candidates)

```bash
just ci-full  # ~10-20 min
```

`just ci-full` is the automated release-confidence lane. It should include the
broader integration surfaces that are too expensive for Tier A, including the
`perl-lsp-ux-tests` first-5-minutes workflow harness.

## Tier C: Real User Confirmation

Manual editor smoke test: diagnostics, completion, hover, go-to-definition, rename

## Metric Definitions

**LSP Metrics** (computed from `features.toml` by `scripts/update-current-status.py`):

| Metric | Formula | Meaning |
| --- | --- | --- |
| **LSP Coverage (user-visible)** | `implemented / trackable` where `counts_in_coverage != false` | Headline metric |
| **Protocol Compliance** | `implemented / trackable` (all features) | Wire-level completeness |

Key terms:

- `implemented` (coverage): Features with `maturity in (ga, production)`
- `trackable` (coverage): Features where `advertised = true`, `maturity != planned`, and `counts_in_coverage != false`
- `implemented` (protocol): Features with `maturity in (ga, production, preview)`
- `trackable` (protocol): Features where `maturity != planned`
- `counts_in_coverage = false`: Protocol plumbing that would otherwise inflate coverage artificially

**Other Metrics**:

- **Corpus counts**: `tree-sitter-perl/test/corpus` sections + `test_corpus/*.pl` files
- **Catalog source**: root `features.toml` is canonical

## Truth Contract

All claims require evidence from:
- `Cargo.toml` (`workspace.package.version`) for the current release line
- `nix develop -c just ci-gate` output
- `bash scripts/ignored-test-count.sh` output
- `features.toml`, capability snapshots, or targeted tests

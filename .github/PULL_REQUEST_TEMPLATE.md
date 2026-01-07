## Summary

<!-- Brief description of changes (1-3 sentences) -->

## Checklist

- [ ] Ran `nix develop -c just ci-gate` locally (or `just ci-gate` if Nix unavailable)
- [ ] Added/updated tests for new functionality
- [ ] Updated relevant documentation (if applicable)

## Optional CI Labels

Add these labels to trigger additional validation (not required for merge).
See `docs/CI_TEST_LANES.md` for the canonical reference.

| Label | What it runs |
|-------|--------------|
| `ci:tests` | Cross-platform test matrix (`test.yml`) |
| `ci:lsp` | LSP integration lane (`lsp-tests.yml`) |
| `ci:property` | Property-based tests (`property-tests.yml`) |
| `ci:strict` | Pedantic clippy lints (`quality-checks.yml`, `rust-strict.yml`) |
| `ci:semver` | API compatibility check (`quality-checks.yml`, `rust-strict.yml`) |
| `ci:determinism` | Test determinism validation (`quality-checks.yml`) |
| `ci:audit` | Security dependency audit (`quality-checks.yml`) |
| `ci:mutation` | Mutation testing (~15-30 min) (`quality-checks.yml`, `ci-expensive.yml`) |
| `ci:bench` | Performance benchmarks (`benchmark.yml`, `lsp-tests.yml`) |
| `ci:coverage` | Test coverage report (`quality-checks.yml`, `lsp-tests.yml`) |
| `ci:all-tests` | Comprehensive suite (`comprehensive_tests.yml`) |
| `ci:docs-truth` | Documentation drift validation (`docs-truth.yml`) |

## Test Plan

<!-- How was this tested? What should reviewers verify? -->

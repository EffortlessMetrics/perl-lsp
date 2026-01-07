## Summary

<!-- Brief description of changes (1-3 sentences) -->

## Checklist

- [ ] Ran `nix develop -c just ci-gate` locally (or `just ci-gate` if Nix unavailable)
- [ ] Added/updated tests for new functionality
- [ ] Updated relevant documentation (if applicable)

## Optional CI Labels

Add these labels to trigger additional validation (not required for merge):

| Label | What it runs |
|-------|--------------|
| `ci:mutation` | Mutation testing (~15-30 min) |
| `ci:bench` | Performance benchmarks |
| `ci:determinism` | Test determinism validation (runs tests 3x) |
| `ci:audit` | Security dependency audit |
| `ci:coverage` | Test coverage report |
| `ci:semver` | API compatibility check |
| `ci:strict` | Pedantic clippy lints |
| `ci:docs-truth` | Documentation drift validation |

## Test Plan

<!-- How was this tested? What should reviewers verify? -->

# perl-lsp Current Status

> **Truth contract**: All claims require evidence from:
> - `nix develop -c just ci-gate` output
> - `bash scripts/ignored-test-count.sh` output
> - Capability snapshots or targeted tests

---

## Verification Protocol

**Tier A: Merge Gate** (required for all merges)
```bash
just ci-gate  # ~2-5 min
```

**Tier B: Release Confidence** (large changes/release candidates)
```bash
just ci-full  # ~10-20 min
```

**Tier C: Real User Confirmation**
Manual editor smoke test: diagnostics, completion, hover, go-to-definition, rename

### Metric Definitions
- **LSP Coverage (GA)**: `advertised_ga / trackable` from `features.toml` (excludes `planned`)
- **Corpus counts**: `tree-sitter-perl/test/corpus` sections + `test_corpus/*.pl` files (fixture counts, not semantic coverage)
- **Catalog source**: Root `features.toml` is canonical

---

## At a Glance

| Metric | Value | Target | Status |
| ------ | ----- | ------ | ------ |
| Tier A Tests | 337 passed, 1 ignored | 100% pass | PASS |
| Tracked Test Debt | 9 (8 bug, 1 manual) | 0 | Near-zero |
| LSP Coverage | 82% (27/33 GA advertised) | 93%+ | In progress |
| Parser Coverage | ~100% | 100% | Complete |
| Semantic Analyzer | Phase 1 (12/12 handlers) | Phase 3 | Core complete |
| Mutation Score | 87% | 87%+ | Target met |
| Documentation | 484 violations | 0 | 8-week plan |

---

## What's True Right Now

- **Parser**: Production-ready with ~100% Perl 5 syntax coverage, 1-150us parsing, 931ns incremental updates
- **LSP Server**: 82% feature coverage, <50ms response times, semantic definition working
- **Semantic Analyzer**: Phase 1 complete with 12/12 critical handlers, `textDocument/definition` integrated
- **Test Infrastructure**: 337 lib tests passing, 4/4 LSP semantic def tests passing
- **Quality**: 87% mutation score, enterprise-grade UTF-16 handling, path validation
- **DAP Server**: Phase 1 bridge to Perl::LanguageServer complete (71/71 tests)

---

## What's Next

1. **CI Pipeline (#211)** - Blocks merge-blocking gates; $720/year savings potential
2. **Sprint B remaining**: #180 (name spans), #181 (workspace features), #191 (document highlighting)
3. **v0.9 Release**: Tag `v0.9.0-semantic-lsp-ready` after docs update
4. **Production v1.0**: #210 (merge gates), #208 (batteries-included UX), #197 (core docs)
5. **Semantic Phase 2/3**: Advanced features (closures, multi-file, imports) - post-v0.9

See [ROADMAP.md](ROADMAP.md) for milestone details.

---

## Known Constraints

- **9 tracked test debt**: 8 bug-related, 1 manual; feature-gated ignores are by design
- **CI Pipeline (#211)**: Blocks merge-blocking gates (#210)
- **484 doc violations**: Infrastructure complete, content phase in progress
- **Semantic Phase 2/3**: Advanced features deferred to post-v0.9

---

## Component Summary

| Component | Status | Notes |
|-----------|--------|-------|
| perl-parser | Production | ~100% Perl 5, 87% mutation score |
| perl-lsp | Production | 82% LSP 3.18 coverage |
| perl-dap | Phase 1 | Bridge mode complete |
| perl-lexer | Production | Context-aware, sub-microsecond |
| perl-corpus | Production | 613 tree-sitter sections + 10 .pl files |

---

## How to Update This File

1. Run `just status-update` to regenerate computed metrics
2. Run `just ci-gate` to verify all gates pass
3. Edit "What's True Right Now" and "What's Next" sections as needed

**Historical archives**: See `docs/archive/status_snapshots/` for sprint logs and completion history.

---

*Last Updated: 2026-01-07*
*Canonical docs: [ROADMAP.md](ROADMAP.md), [features.toml](../features.toml)*

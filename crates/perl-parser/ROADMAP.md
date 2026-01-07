# Parser Roadmap

> **This file has been superseded**. See the canonical roadmap at [`docs/ROADMAP.md`](../../docs/ROADMAP.md).
>
> Historical version archived at [`docs/archive/roadmaps/PARSER_ROADMAP_2025-01-19.md`](../../docs/archive/roadmaps/PARSER_ROADMAP_2025-01-19.md).

## Quick Status (2026-01-07)

| Component | Status | Details |
|-----------|--------|---------|
| perl-parser v3 | Production | 100% Perl syntax coverage, <1ms incremental parsing |
| Semantic Analyzer | Phase 1 Complete | 12/12 critical node handlers |
| LSP Server | ~91% Ready | textDocument/definition using semantic analysis |

**Canonical gate**: `nix develop -c just ci-gate`

For the full roadmap, release schedule, and detailed component status, see [`docs/ROADMAP.md`](../../docs/ROADMAP.md).

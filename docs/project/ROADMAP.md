# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current release line is taken from [`../../Cargo.toml`](../../Cargo.toml); current capability truth is taken from [`../../features.toml`](../../features.toml).
> Generated sections between `<!-- BEGIN: ... -->` and `<!-- END: ... -->` are machine-updated by `just roadmap-update`.

## Current Framing

- Current release line: `v0.11.0` public alpha
- Active milestone: `v0.12.0` public-alpha hardening sprint
- Canonical local receipt: `nix develop -c just ci-gate`

## How To Read This File

- [CURRENT_STATUS.md](CURRENT_STATUS.md) tells you what is true right now.
- This roadmap tells you what we are trying to land next.
- [../../ROADMAP.md](../../ROADMAP.md) and [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) are summaries, not the canonical plan.

## Current Release Line: v0.11.0 Public Alpha

The current shipped line is still public alpha. The repo is usable today, but APIs, protocol details, and packaging can still change between minor releases.

Recent shipped work in the `v0.10.x` to `v0.11.x` alpha line:

- Initial public-alpha packaging and marketplace preparation
- Release orchestration and topological publish validation
- Continued parser, workspace, and LSP microcrate extraction
- Security hardening, validation receipts, and docs restructuring

## Active Milestone: v0.12.0 Public-Alpha Hardening Sprint

This milestone is about making the current alpha line more credible on real-world Perl without pretending the stability contract already exists.

### Main tracks

- **Corpus and ratchets**: commit and ratchet the CPAN baseline, keep system/common corpus receipts green
- **Parser robustness**: land Wave 2-4 parser fixes, keep edge-case and hang-risk suites bounded
- **Semantic framework coverage**: Moo, Moose, Class::Accessor, `use parent` / `use base`, and export-list-aware resolution
- **Editor and debugger hardening**: keep LSP and DAP flows solid while parser work lands
- **Documentation and release alignment**: make top-level docs, status, roadmap, and release posture stop contradicting each other

### Exit criteria

- [ ] `.ci/cpan-corpus-baseline.json` is committed and ratcheted
- [ ] CPAN top-1000 clean parse rate reaches `90%+`
- [ ] Internal edge-case, parser-stress, and hang-risk suites stay green
- [ ] No hang, crash, or stack-overflow regressions appear in corpus sweeps
- [ ] Common corpus remains strict zero-error and only grows
- [ ] CPAN known-clean manifest grows without regressions once seeded
- [ ] Moo/Moose/Class::Accessor coverage reaches the maintained test targets
- [ ] Cross-file inheritance resolution via `use parent` / `use base` lands
- [ ] Exporter/Sub::Exporter-style export-list parsing improves semantic resolution
- [ ] `nix develop -c just ci-gate` stays green through the hardening push

### Supporting docs

- [CURRENT_STATUS.md](CURRENT_STATUS.md)
- [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md)
- [CPAN_CORPUS_STRATEGY.md](CPAN_CORPUS_STRATEGY.md)

## Now / Next / Later

### Now

- Raise CPAN clean-parse coverage while keeping ratchets honest
- Finish semantic framework work needed for real-world Perl projects
- Keep the release surface and docs aligned with the current alpha line

### Next

- Diagnostic hardening: `strict`, `warnings`, dead-code signals, and safe static analysis
- Refactoring reliability: safer rename/extract workflows and broader test coverage
- DAP hardening beyond the current preview posture

### Later

- `v0.15.0`: stability contract for APIs and advertised wire behavior
- Platform certification and broader distribution packaging
- Performance, security, and API stabilization work toward `v1.0.0`

## Milestone Ladder

### v0.12.0

Public-alpha hardening sprint across parser quality, semantic framework coverage, docs alignment, and release receipts.

### v0.13.0

Diagnostic hardening and safer static analysis without executing project code.

### v0.14.0

Refactoring and debugger hardening: safer rewrite operations and deeper native DAP support.

### v0.15.0

The stability contract: clearer support posture, stronger compatibility expectations, and tighter release discipline.

### Beyond v0.15.0

- Performance hardening for larger workspaces
- Security posture and documentation hardening
- API stabilization and migration guidance
- Path to `v1.0.0`

## LSP Feature Implementation

The LSP compliance table is auto-generated from `features.toml`.

<!-- BEGIN: COMPLIANCE_TABLE -->
| Area | Implemented | Total | Coverage |
|------|-------------|-------|----------|
| debug | 10 | 10 | 100% |
| notebook | 2 | 2 | 100% |
| protocol | 9 | 9 | 100% |
| text_document | 41 | 41 | 100% |
| window | 9 | 9 | 100% |
| workspace | 26 | 26 | 100% |
| **Overall** | **97** | **97** | **100%** |
<!-- END: COMPLIANCE_TABLE -->

For live capability posture, run `just status-check` or read [CURRENT_STATUS.md](CURRENT_STATUS.md).

## Truth Sources

| Topic | Source |
| --- | --- |
| Current release line | [`../../Cargo.toml`](../../Cargo.toml) |
| Capability catalog | [`../../features.toml`](../../features.toml) |
| Evidence-backed metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) |
| Top-level summary docs | [../../ROADMAP.md](../../ROADMAP.md), [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) |

<!-- Last Updated: 2026-03-19 -->

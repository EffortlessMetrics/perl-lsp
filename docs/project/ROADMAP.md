# perl-lsp Roadmap

> Canonical planning document.
> Evidence and computed metrics belong in [CURRENT_STATUS.md](CURRENT_STATUS.md).
> Current workspace version is taken from [`../../Cargo.toml`](../../Cargo.toml);
> published release state must be verified against GitHub Releases;
> current capability truth is taken from [`../../features.toml`](../../features.toml).

## Current Framing

- Workspace version line: `v0.12.1`
- Latest published release: `v0.12.0` (verified 2026-03-30)
- Active release target: `v0.12.1` fix-forward cut
- Canonical local receipt: `nix develop -c just ci-gate`

## How To Read This File

- [CURRENT_STATUS.md](CURRENT_STATUS.md) tells you what is true right now.
- This roadmap tells you what we are trying to land next.
- [../../ROADMAP.md](../../ROADMAP.md) and [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) are summaries, not the canonical plan.

## Current Release Target: v0.12.1 Fix-Forward Prep

`main` is now version-bumped to `v0.12.1`. The latest published GitHub release
is `v0.12.0`, verified on 2026-03-30. This roadmap tracks the narrow
fix-forward cut needed after the initial public alpha release so the repo front
door, hook hygiene, and operator surfaces stay aligned with the shipped
artifact line.

Recent shipped work in the published `v0.12.0` line:

- Initial public alpha GitHub release with native `perllsp` assets and VSIX
- Package rename split across `perllsp` and `perl-lsp-rs`
- Release orchestration, topological publish validation, and launch-day docs cleanup
- Continued parser, workspace, and LSP microcrate extraction and hardening

## Active Milestone: v0.12.1 Fix-Forward Release Prep

This milestone is about closing the launch regressions that surfaced right after
`v0.12.0`, keeping the receipts green, and cutting `v0.12.1` without reopening
the broader alpha scope.

### Main tracks

- **Release surface repair**: keep the README, release notes, install guidance,
  and asset examples aligned with the actual `perllsp` / `perl-lsp-rs` shipped
  surfaces
- **Hook hygiene**: keep hook-test fixtures isolated from the real checkout and
  block placeholder identities before they can leak into local commit metadata
- **Packaging follow-through**: keep Cargo, VS Code, docs, and operator runbooks
  aligned around `v0.12.1` while `v0.12.0` remains the latest published GitHub release
- **Post-launch stability**: keep `nix develop -c just ci-gate` and the release
  receipts green while the fix-forward cut is prepared

### Exit criteria

- [ ] `Cargo.toml`, `features.toml`, and `vscode-extension/package.json` all report `0.12.1`
- [ ] `CHANGELOG.md` contains a dated `## [0.12.1]` entry and leaves `[Unreleased]` empty
- [ ] The top-level README and release docs no longer drift from the shipped `perllsp` asset line
- [ ] Hook-test fixtures cannot mutate repo-local git identity or front-door files in the real checkout
- [ ] `nix develop -c just ci-gate` stays green through the `v0.12.1` fix-forward prep
- [ ] The `v0.12.1` release flow completes without tag, package-name, or install-surface drift

### Supporting docs

- [CURRENT_STATUS.md](CURRENT_STATUS.md)
- [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md)
- [CPAN_CORPUS_STRATEGY.md](CPAN_CORPUS_STRATEGY.md)

## Now / Next / Later

### Now

- Close the launch regressions discovered after the `v0.12.0` tag without reopening the wider alpha scope
- Keep package names, install guidance, and operator docs aligned with `perllsp` and `perl-lsp-rs`
- Finish the `v0.12.1` fix-forward cut with clean release receipts and no more front-door drift

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

Initial public alpha release across parser quality, semantic framework coverage, docs alignment, and release receipts.

### v0.12.1

Fix-forward release to close launch regressions in README, hook-fixture isolation,
git-hook installation, and release-surface alignment after the initial public alpha cut.

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
| text_document | 42 | 42 | 100% |
| window | 9 | 9 | 100% |
| workspace | 26 | 26 | 100% |
| **Overall** | **98** | **98** | **100%** |
<!-- END: COMPLIANCE_TABLE -->

For live capability posture, run `just status-check` or read [CURRENT_STATUS.md](CURRENT_STATUS.md).

## Truth Sources

| Topic | Source |
| --- | --- |
| Workspace version line | [`../../Cargo.toml`](../../Cargo.toml) |
| Latest published release | GitHub Releases |
| Capability catalog | [`../../features.toml`](../../features.toml) |
| Evidence-backed metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) |
| Top-level summary docs | [../../ROADMAP.md](../../ROADMAP.md), [../../NOW_NEXT_LATER.md](../../NOW_NEXT_LATER.md) |

<!-- Last Updated: 2026-03-28 -->

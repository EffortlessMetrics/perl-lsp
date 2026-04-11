# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

Use this file to see what the project is trying to land next. Use the canonical
project docs when you need exact release facts, receipts, or milestone detail.

## State References

- Active milestone plan: [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Current truth and receipts: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)
- Published release tracking: [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Now (post-v0.12.3 ship / pre-announcement cleanup)

- `v0.12.3` shipped to GitHub Releases, VS Code Marketplace, and Open VSX on 2026-04-09
- crates.io intentionally remains on `0.12.2` while the registry window is still deferred
- Pre-announcement plumbing: license badge fix, Docker arm64 timeout fix, dependency triage, harness archival, SRP microcrate extractions
- Distribution channel verification across GitHub Releases, VS Code Marketplace, Open VSX, Docker Hub, and the delayed crates.io line
- Coroutine request #3539 is being re-scoped: defer speculative core syntax support and split follow-up into upstream-status tracking plus CPAN API IDE support
- See [docs/project/ROADMAP.md](docs/project/ROADMAP.md) "Now (post-v0.12.3 / pre-v0.13.0)" for the active item list

## Next (v0.13.0 — public alpha announcement)

- 0.12.x line built confidence across parser, diagnostics, refactoring, distribution, AI inline completion
- Quality cleanup PRs land, version bump to 0.13.0
- Seamless install story verified across all distribution channels
- Announcement blog post / release notes

## Beyond v0.13.0

- Stability contract for APIs and advertised wire behavior
- Performance hardening for larger workspaces
- Security posture and documentation hardening
- Path to `v1.0.0`

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

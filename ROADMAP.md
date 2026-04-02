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

## Now (v0.12.2 — stability hardening)

- Merge CI improvement PRs (#3078–#3080) and Dependabot batch (#3064–#3071)
- Fix pre-push hook branch-deletion regression (#3081)
- Scout top 3 uninvestigated parser blockers and raise CPAN clean rate toward 80%
- Close error-handling hygiene batch and test gap issues

## Next (v0.12.3 — diagnostic & refactoring hardening)

- Dead code highlighting, perlcritic integration, `strict`/`warnings` diagnostics
- Workspace-scoped rename, extract variable/subroutine, subroutine inlining
- Moose/Moo framework support (method modifiers, role composition)
- DAP Phase 3 test suite and cross-platform signal handling

## Then

- `v0.12.4`: parser corpus confidence (≥85% CPAN clean) and performance profiling
- `v0.12.5`: distribution and packaging (Docker, Nix, Homebrew, Windows/Linux package managers)

## Later (v0.13.0 — public alpha announcement)

- 0.12.x builds confidence; 0.13.0 is the initial public alpha announcement
- Seamless install story across all distribution channels
- Performance, security, and compatibility hardening on the path to `v1.0.0`

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

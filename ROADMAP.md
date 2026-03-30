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

## Now

- Keep release validation green while `v0.12.1` closes the launch regressions discovered after `v0.12.0`
- Keep install guidance and package naming aligned with the actual `perllsp` / `perl-lsp-rs` surfaces
- Raise parser and corpus confidence without hiding regressions behind broader receipts
- Finish framework-aware semantic work for real-world Perl projects
- Keep README, docs, changelog, and release guidance aligned so users do not have to infer the release state

## Next

- Harden diagnostics, refactoring, and debugger behavior after the alpha cut
- Follow through on distribution and packaging cleanup once the release line is live
- Keep shrinking the gap between crate-level docs, editor docs, and release operations

## Later

- `v0.15.0` stability contract for APIs and advertised wire behavior
- Broader packaging and platform certification
- Performance, security, and compatibility hardening on the path to `v1.0.0`

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

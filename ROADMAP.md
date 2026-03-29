# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Current Framing

- Workspace version line: `v0.12.0`
- Latest published GitHub release: `v0.11.0` (verified 2026-03-29)
- Active milestone: `v0.12.0` initial public alpha cut
- Canonical plan: [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Current truth and receipts: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)

## Now

- Keep release validation green while `v0.12.0` moves from version line to shipped release
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

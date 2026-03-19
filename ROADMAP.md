# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## Current Framing

- Current release line: `v0.11.0` public alpha (`Cargo.toml` `workspace.package.version`)
- Active milestone: `v0.12.0` public-alpha hardening sprint
- Current priorities: parser-quality ratchets, semantic-framework coverage, DAP/LSP hardening, and documentation alignment

## Now

- Raise the CPAN baseline and keep parser boundedness receipts green
- Land semantic framework work for Moo, Moose, `use parent` / `use base`, and export-list-aware resolution
- Keep release and validation flows stable while the hardening work lands
- Align top-level documentation so README, roadmap, status, and agent guidance stop contradicting each other

## Next

- Diagnostic hardening around `strict`, `warnings`, dead-code signals, and safe analysis
- Refactoring reliability and debugger hardening beyond the current preview posture
- Distribution and release-surface cleanup after the public-alpha hardening sprint

## Later

- `v0.15.0` stability contract for APIs and advertised wire behavior
- Platform certification and broader distribution packaging
- Performance, security, and API-stability hardening on the path to `v1.0.0`

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. If it starts carrying detailed receipts or large milestone tables again, move that detail back to the canonical project docs.

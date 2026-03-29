# NOW / NEXT / LATER

> Last Updated: 2026-03-28
> Workspace version line: `v0.12.0`
> Latest published release: `v0.11.0` (verified 2026-03-28)
> Active milestone: `v0.12.0` initial public alpha release prep

This file is the short planning snapshot. The detailed roadmap is [docs/project/ROADMAP.md](docs/project/ROADMAP.md). The evidence-backed status view is [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

## NOW

- Parser hardening: raise the CPAN baseline, keep boundedness receipts green, and land Wave 2-4 parser fixes
- Semantic framework coverage: Moo, Moose, Class::Accessor, `use parent` / `use base`, and export-list-aware resolution
- Release and tooling hygiene: keep `nix develop -c just ci-gate` green while initial public alpha prep lands
- Documentation alignment: keep README, roadmap, status, and install guidance consistent about workspace version versus latest published release

## NEXT

- Diagnostic hardening around `strict`, `warnings`, safe static analysis, and dead-code signals
- Refactoring and debugger reliability beyond the current preview posture
- Broader release-surface cleanup after the `v0.12.0` initial public alpha cut

## LATER

- `v0.15.0` stability contract for APIs and advertised behavior
- Platform certification and packaging expansion
- Performance, security, and API-stability work on the path to `v1.0.0`

## Working Rules

- Keep “current release line” separate from “next milestone”.
- Put receipts and computed metrics in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md), not here.
- Put detailed milestone criteria in [docs/project/ROADMAP.md](docs/project/ROADMAP.md), not here.

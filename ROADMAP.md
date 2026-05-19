# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

Use this file to see what the project is trying to land next. Use the canonical
project docs when you need exact release facts, receipts, or milestone detail.

## State References

- Active milestone plan: [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Current truth and receipts: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)
- Release readiness and channel proof: [docs/project/status/release.md](docs/project/status/release.md)
- Compiler-backed LSP build-out: [docs/project/COMPILER_BACKED_LSP_ROADMAP.md](docs/project/COMPILER_BACKED_LSP_ROADMAP.md)
- CI/control-plane wave: [docs/project/CI_WAVE_EXECUTION_PLAN.md](docs/project/CI_WAVE_EXECUTION_PLAN.md)
- Editor-trust wave: [docs/project/EDITOR_TRUST_WAVE.md](docs/project/EDITOR_TRUST_WAVE.md)
- Provider cutover dashboard: [docs/project/status/provider_cutover.md](docs/project/status/provider_cutover.md)
- Real Perl editor trust dashboard: [docs/project/status/real_perl_editor_trust_v1.md](docs/project/status/real_perl_editor_trust_v1.md)
- Published release tracking: [RELEASE_HISTORY.md](RELEASE_HISTORY.md) and [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Now (v0.14.0 public-alpha channel closeout)

- `v0.14.0` is the current public-alpha release line; verify live channel state before citing completion
- GitHub Release and crates.io surfaces show `v0.14.0` live, while channel closeout still needs explicit receipts across Docker, VS Code Marketplace, Open VSX, and the owned Homebrew tap path
- Keep package-version language separate from product-posture language: SemVer package version, public-alpha product promise
- CI/control-plane work is sequenced through narrow lanes in [docs/project/CI_WAVE_EXECUTION_PLAN.md](docs/project/CI_WAVE_EXECUTION_PLAN.md), starting with `update-status --write` streaming
- See [docs/project/ROADMAP.md](docs/project/ROADMAP.md) for the canonical active item list, exit criteria, and post-release sequencing

## Next (post-v0.14.0)

- Close release-channel receipts before starting broad cleanup
- Continue compiler-backed provider cutovers with source/freshness/provenance receipts and live fallback behavior
- Resume parser, corpus, semantic, DAP, and editor-trust hardening through one-lane, one-PR acceptance receipts
- Keep the install story verified across all distribution channels and keep release notes tied to concrete receipts

## Beyond v0.14.0

- Stability contract for APIs and advertised wire behavior
- Performance hardening for larger workspaces
- Security and supply-chain posture hardening
- Path to `v1.0.0`

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

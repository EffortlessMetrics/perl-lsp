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
- Provider cutover dashboard: [docs/project/status/provider_cutover.md](docs/project/status/provider_cutover.md)
- Real Perl editor trust dashboard: [docs/project/status/real_perl_editor_trust_v1.md](docs/project/status/real_perl_editor_trust_v1.md)
- Published release tracking: [RELEASE_HISTORY.md](RELEASE_HISTORY.md) and [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Now (v0.14.0 public-alpha patch prep)

- Workspace version line is `v0.14.0`; verify live channel state before claiming a release is published.
- Release proof is the top lane: close the v0.14.0 prep checks, keep crates.io/editor/Docker/Homebrew receipts wired to the release runbook, and keep public language at public alpha.
- CI/control-plane work is the next execution lane: streaming status output, CI trigger lint, normalized expected-skip/stale-check states, reconciler label projection, disposition evidence, merge-train receipts, and tokmd advisory calibration.
- Compiler-backed LSP work continues through fact-source tracing, shadow/live comparison, provider cutover dashboards, and explicit fallback behavior.
- Real-project editor trust is the user-facing promotion gate for provider changes; fixtures prove implementation, but dashboarded workflows justify release claims.

## Next (post-v0.14.0)

- Resume parser, corpus, semantic, DAP, and distribution hardening after release-channel receipts close.
- Run the Editor Trust Wave as one lane with one canonical checklist and one verification receipt.
- Keep install guidance verified across GitHub Releases, crates.io, VS Code Marketplace, Open VSX, Docker, and the owned Homebrew tap path.
- Keep release notes concise, receipt-backed, and explicit about public-alpha status.

## Later (toward v1.0.0)

- Stability contract for public APIs and advertised wire behavior.
- Performance hardening for larger workspaces.
- Security posture and documentation hardening.
- A measured path from public alpha to `v1.0.0` based on channel reliability and editor-trust evidence.

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

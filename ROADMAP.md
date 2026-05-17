# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

Use this file to see what the project is trying to land next. Use the canonical
project docs when you need exact release facts, receipts, or milestone detail.

## State References

- Active milestone plan: [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Current truth and receipts: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)
- Compiler-backed LSP build-out: [docs/project/COMPILER_BACKED_LSP_ROADMAP.md](docs/project/COMPILER_BACKED_LSP_ROADMAP.md)
- Editor-trust wave: [docs/project/EDITOR_TRUST_WAVE.md](docs/project/EDITOR_TRUST_WAVE.md)
- CI/control-plane wave: [docs/project/CI_WAVE_EXECUTION_PLAN.md](docs/project/CI_WAVE_EXECUTION_PLAN.md)
- Published release tracking: [RELEASE_HISTORY.md](RELEASE_HISTORY.md)

## Now (v0.14.0 public-alpha patch prep)

- Workspace version line is `v0.14.0`; release dispatch remains intentionally pending until prep checks pass.
- Keep install and release language explicit: public alpha, not stable/GA.
- Finish release-channel proof for GitHub Release, crates.io, Docker, VS Code Marketplace, Open VSX, and the owned Homebrew tap path.
- Keep release notes concise and tied to concrete channel receipts.
- Run the CI/control-plane follow-up wave as independent, reviewable slices instead of a broad redesign.
- Continue compiler-backed provider cutover only through measured, fact-source-traced proof lanes.
- See [docs/project/ROADMAP.md](docs/project/ROADMAP.md) for exit criteria, sequencing, and guardrails.

## Next (post-v0.14.0)

- Resume parser, corpus, semantic, DAP, and editor-trust hardening after the release-channel receipts close.
- Promote only proven scorecard floors from nightly or label-gated checks into merge-blocking checks.
- Keep module-resolution and `@INC` behavior consistent across completion, definition, hover, diagnostics, and workspace symbols.
- Continue compiler-backed LSP cutovers by provider, with live fallback and rollback receipts.

## Later

- Stability contract for APIs and advertised wire behavior.
- Performance hardening for larger workspaces.
- Security posture and documentation hardening.
- Path to `v1.0.0`.

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

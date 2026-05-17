# perl-lsp Roadmap

> This top-level file is the short roadmap entrypoint.
> The canonical planning document is [docs/project/ROADMAP.md](docs/project/ROADMAP.md).
> Evidence and current receipts live in [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md).

Use this file to see what the project is trying to land next. Use the canonical
project docs when you need exact release facts, receipts, or milestone detail.

## State References

- Active milestone plan: [docs/project/ROADMAP.md](docs/project/ROADMAP.md)
- Current truth and receipts: [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)
- v0.14.0 release notes and claim boundary: [docs/releases/v0.14.0.md](docs/releases/v0.14.0.md)
- Compiler-backed LSP build-out: [docs/project/COMPILER_BACKED_LSP_ROADMAP.md](docs/project/COMPILER_BACKED_LSP_ROADMAP.md)
- Editor trust wave: [docs/project/EDITOR_TRUST_WAVE.md](docs/project/EDITOR_TRUST_WAVE.md)
- CI/control-plane wave: [docs/project/CI_WAVE_EXECUTION_PLAN.md](docs/project/CI_WAVE_EXECUTION_PLAN.md)
- Published release tracking: [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases)

## Now (v0.14.0 public-alpha patch prep)

- Workspace version line is `v0.14.0`; keep public language as public alpha, not stable/GA
- Finish RP-2 dry-run publish readiness before tag, publish, or announcement operations
- Verify install-channel receipts across crates.io, GitHub Release assets, Docker, VS Code Marketplace, Open VSX, and Homebrew
- Keep the release notes tied to shipped receipts and move deferred work to successor issues instead of expanding the train
- Continue the top CI/control-plane urgency lane: `update-status --write` progress streaming and failure attribution

## Next (post-v0.14.0)

- Resume compiler-backed provider cutovers only where provenance and fallback receipts show equal-or-better behavior
- Run the editor-trust wave through one canonical lane per UX area, with deterministic receipts before CI ratchets
- Continue quality cleanup in focused crate-sized PRs: production panic/unwrap audit, temporary allow burndown, dependency triage, and install-surface verification
- Keep release guidance copy-pasteable for users while preserving the public-alpha claim boundary

## Later (v1.0 runway)

- Define the stable Rust API surface versus internal implementation crates
- Define advertised LSP/DAP wire behavior and fallback compatibility promises
- Establish large-workspace performance and memory budgets that can run safely in agent worktrees
- Document security expectations for subprocess execution, Perl environment construction, and extension distribution
- Set a `v1.0.0` target only after the API, wire-behavior, performance, and security contracts have reviewable acceptance criteria

## Update Rules

- Update [docs/project/ROADMAP.md](docs/project/ROADMAP.md) when milestone framing changes.
- Update [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) with `just status-update` and `just status-check` when generated metrics move.
- Keep this file short. Detailed receipts, milestone criteria, and subsystem metrics belong in the canonical project docs.

# perl-lsp Current Status

> This file is a stable landing page for backward compatibility.
> Computed metrics have moved to modular subsystem files under `docs/project/status/`.
> See [status/index.md](status/index.md) for the full overview.

## Quick Links

| What you need | Where to find it |
| --- | --- |
| Project overview & narrative | [status/index.md](status/index.md) |
| LSP coverage & compliance | [status/lsp.md](status/lsp.md) |
| Test counts & tracked debt | [status/tests.md](status/tests.md) |
| Parser corpus & coverage | [status/parser.md](status/parser.md) |
| Quality metrics | [status/quality.md](status/quality.md) |
| Editor UX planning scaffold | [status/editor_ux.json](status/editor_ux.json) |
| Release readiness & blockers | [status/release.md](status/release.md) |
| Verification protocol | [protocols/verification.md](protocols/verification.md) |
| Planning & roadmap | [ROADMAP.md](ROADMAP.md) |

## At a Glance

| Metric | Value | Source |
| --- | --- | --- |
| **Workspace version line** | `v0.13.0-rc1` | [`Cargo.toml`](../../Cargo.toml) |
| **Latest release candidate** | `v0.13.0-rc1`, 2026-04-30 | GitHub Releases, crates.io, Docker Hub |
| **crates.io line** | `0.13.0-rc1` across 32 published crates | `[workspace.metadata.publish.allow]` |
| **Editor marketplace line** | Non-prerelease `0.13.0` pending | VS Marketplace requires non-prerelease SemVer; Open VSX must publish independently |
| **Release history** | [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) | Canonical cross-channel ledger |
| **Active milestone** | `v0.13.0` public alpha launch | [status/index.md](status/index.md) |
| **Merge gate** | `nix develop -c just ci-gate` | [protocols/verification.md](protocols/verification.md) |
| **LSP Coverage** | See [status/lsp.md](status/lsp.md) | Generated per-merge |
| **Test counts** | See [status/tests.md](status/tests.md) | Generated per-merge |
| **Parser coverage** | See [status/parser.md](status/parser.md) | Generated per-merge |
| **Quality metrics** | See [status/quality.md](status/quality.md) | Generated per-merge |
| **Editor UX planning scaffold** | See [status/editor_ux.json](status/editor_ux.json) | Generated per-merge |

## How to Update Metrics

```bash
just status-update            # regenerate all 4 subsystem files plus the UX planning scaffold
just status-update lsp        # regenerate only LSP metrics (fast)
just status-check             # verify subsystem files are current
```

*Generated subsystem files are auto-updated post-merge by `.github/workflows/post-merge-status.yml`.*
*Narrative files (`status/index.md`, `status/release.md`) are human-owned and stable.*

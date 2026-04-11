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
| Quality metrics | [status/quality.md](status/quality.md) + [status/editor_ux.json](status/editor_ux.json) |
| Release readiness & blockers | [status/release.md](status/release.md) |
| Verification protocol | [protocols/verification.md](protocols/verification.md) |
| Planning & roadmap | [ROADMAP.md](ROADMAP.md) |

## At a Glance

| Metric | Value | Source |
| --- | --- | --- |
| **Workspace version line** | `v0.12.3` | [`Cargo.toml`](../../Cargo.toml) |
| **Latest GitHub/editor release** | `v0.12.3`, 2026-04-09 | GitHub Releases, VS Code Marketplace, Open VSX |
| **crates.io line** | `v0.12.2`, 2026-04-07 | crates.io |
| **Active milestone** | `v0.13.0` public alpha announcement | [status/index.md](status/index.md) |
| **Merge gate** | `nix develop -c just ci-gate` | [protocols/verification.md](protocols/verification.md) |
| **LSP Coverage** | See [status/lsp.md](status/lsp.md) | Generated per-merge |
| **Test counts** | See [status/tests.md](status/tests.md) | Generated per-merge |
| **Parser coverage** | See [status/parser.md](status/parser.md) | Generated per-merge |
| **Quality metrics** | See [status/quality.md](status/quality.md) + [status/editor_ux.json](status/editor_ux.json) | Generated per-merge |

## How to Update Metrics

```bash
just status-update            # regenerate all 4 subsystem files plus the UX receipt
just status-update lsp        # regenerate only LSP metrics (fast)
just status-check             # verify subsystem files are current
```

*Generated subsystem files are auto-updated post-merge by `.github/workflows/post-merge-status.yml`.*
*Narrative files (`status/index.md`, `status/release.md`) are human-owned and stable.*

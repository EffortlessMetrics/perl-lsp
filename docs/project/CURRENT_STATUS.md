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
| Semantic capability dashboard | [status/semantic_capability_dashboard.md](status/semantic_capability_dashboard.md) |
| Real Perl Editor Trust v1 dashboard | [status/real_perl_editor_trust_v1.md](status/real_perl_editor_trust_v1.md) |
| Editor UX planning scaffold | [status/editor_ux.json](status/editor_ux.json) |
| Compiler-backed LSP roadmap | [COMPILER_BACKED_LSP_ROADMAP.md](COMPILER_BACKED_LSP_ROADMAP.md) |
| Release readiness & blockers | [status/release.md](status/release.md) |
| Verification protocol | [protocols/verification.md](protocols/verification.md) |
| Planning & roadmap | [ROADMAP.md](ROADMAP.md) |

## At a Glance

| Metric | Value | Source |
| --- | --- | --- |
| **Workspace version line** | `v0.15.0` | [`Cargo.toml`](../../Cargo.toml) |
| **Current release train** | `v0.15.2` latest (2026-05-26); `v0.16.0` is next | [CHANGELOG.md](../../CHANGELOG.md) |
| **Published crate surface** | 31 crates | [`[workspace.metadata.publish.allow]`](../../Cargo.toml) |
| **Release history** | [RELEASE_HISTORY.md](../../RELEASE_HISTORY.md) | Canonical cross-channel ledger |
| **Active milestone** | `v0.16.0` planning | [status/index.md](status/index.md) |
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

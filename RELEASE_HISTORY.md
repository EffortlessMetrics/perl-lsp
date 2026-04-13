# Release History

Canonical release ledger for [perl-lsp](https://github.com/EffortlessMetrics/perl-lsp).

This file is:
- **Backwards-looking** — records what shipped, not what is planned
- **Append-only** — rows are not rewritten; corrections are additive with notes
- **Cross-channel** — tracks GitHub assets, crates.io, editor marketplaces, and containers

GitHub Release `publishedAt` is used as the release date when available.
Tag commit timestamps may differ from release dates.

## Release ledger

| Version | Tag | GitHub Release | Released | Tag commit | Compare | Assets | crates.io | VS Code Marketplace | Notes file |
|---------|-----|----------------|----------|------------|---------|--------|-----------|---------------------|------------|
| [0.12.4] | `v0.12.4` | [yes][gh-0.12.4] | 2026-04-12 | `5ebb37aa` | [v0.12.3...v0.12.4] | 9 (7 binaries, SHA256SUMS, SBOM) | deferred | [perl-lsp-rs][vsce] | [v0.12.4][n-0.12.4] |
| [0.12.3] | `v0.12.3` | [yes][gh-0.12.3] | 2026-04-09 | `a86af221` | [v0.12.2...v0.12.3] | 10 (+ VSIX) | deferred | [perl-lsp-rs][vsce] | [v0.12.3][n-0.12.3] |
| [0.12.2] | `v0.12.2` | [yes][gh-0.12.2] | 2026-04-04 | `1c0620d8` | [v0.12.1...v0.12.2] | 9 | 0.12.2 (2026-04-08) | [perl-lsp-rs][vsce] | [v0.12.2][n-0.12.2] |
| [0.12.1] | `v0.12.1` | [yes][gh-0.12.1] | 2026-03-31 | `7e8984b5` | [v0.12.0...v0.12.1] | 10 (+ VSIX) | 0.12.1 (2026-04-01) | [perl-lsp-rs][vsce] | [v0.12.1][n-0.12.1] |
| [0.12.0] | `v0.12.0` | [yes][gh-0.12.0] | 2026-03-30 | `4c909c2d` | [v0.11.0...v0.12.0] | 10 (+ VSIX) | — | [perl-lsp-rs][vsce] | [v0.12.0][n-0.12.0] |
| [0.11.0] | `v0.11.0` | [yes][gh-0.11.0] | 2026-03-12 | `d22ac734` | [v0.8.5...v0.11.0] | 11 (+ 2 VSIX) | — | [perl-lsp-rs][vsce] | [v0.11.0][n-0.11.0] |
| [0.10.0] | — | — | 2026-02-28 (CL) | — | — | — | — | — | [v0.10.0][n-0.10.0] |
| [0.9.1] | `v0.9.1` | — | 2026-02-20 (tag) | `c82a1604` | — | — | — | — | [v0.9.1][n-0.9.1] |
| [0.9.0] | — | — | 2026-01-18 (CL) | — | — | — | — | — | [v0.9.0][n-0.9.0] |
| [0.8.8] | — | — | 2025-12-01 (CL) | — | — | — | — | — | [v0.8.8][n-0.8.8] |
| [0.8.5] | `v0.8.5` | [yes][gh-0.8.5] | 2025-08-24 | `ae75da03` | [v0.8.3...v0.8.5] | 2 (linux-x64 + checksum) | — | — | [v0.8.5][n-0.8.5] |
| 0.8.3-rc1 | `v0.8.3-rc1` | — | 2025-08-15 (tag) | `150a22b1` | — | — | — | — | — |
| [0.8.3] | `v0.8.3` | [yes][gh-0.8.3] | 2025-08-23 | `5331007a` | — | 0 (source-only) | — | — | [v0.8.3][n-0.8.3] |
| 0.8.2 | `v0.8.2` | — | 2025-08-12 (tag) | `0b962684` | — | — | — | — | — |
| 0.8.0 | `v0.8.0` | — | 2025-08-11 (tag) | `2eeb06c5` | — | — | — | — | — |
| 0.7.3 | `v0.7.3` | — | 2025-08-06 (tag) | `20751374` | — | — | — | — | — |
| 0.7.2 | `v0.7.2` | — | 2025-08-06 (tag) | `a19ba90b` | — | — | — | — | — |
| 0.5.0 | `v0.5.0` | — | 2025-08-03 (tag) | `60190640` | — | — | — | — | — |
| 0.1.0-pest | `v0.1.0-pest` | — | 2025-07-20 (tag) | `4f92dc57` | — | — | — | — | — |

### Legend

- **"—"** = does not exist / not applicable
- **"deferred"** = release published to GitHub but crates.io publish intentionally postponed
- **"(CL)"** = date from CHANGELOG only (no tag or release exists)
- **"(tag)"** = date from tag commit (no GitHub Release exists)
- Versions without a tag or GitHub Release are CHANGELOG-only scope markers that never shipped as distinct artifacts
- The v0.11.0 release included two VSIX files (`perl-lsp-0.11.0.vsix` and `perl-lsp-rs-0.11.0.vsix`) due to the extension rename

### Eras

| Era | Versions | Period | Focus |
|-----|----------|--------|-------|
| **Pest parser** | 0.1.0-pest — 0.5.0 | Jul — Aug 2025 | PEG grammar, initial AST |
| **Native parser + LSP** | 0.7.x — 0.8.x | Aug 2025 | Recursive descent parser, first LSP features, first public releases |
| **Feature buildout** | 0.9.x — 0.10.0 | Jan — Feb 2026 | Semantic analyzer, DAP, release orchestration |
| **Announcement prep** | 0.11.0 — 0.12.x | Mar — Apr 2026 | Multi-platform binaries, VS Code Marketplace, crates.io |

## Links

<!-- Notes files -->
[n-0.12.4]: docs/releases/v0.12.4.md
[n-0.12.3]: docs/releases/v0.12.3.md
[n-0.12.2]: docs/releases/v0.12.2.md
[n-0.12.1]: docs/releases/v0.12.1.md
[n-0.12.0]: docs/releases/v0.12.0.md
[n-0.11.0]: docs/releases/v0.11.0.md
[n-0.10.0]: docs/releases/v0.10.0.md
[n-0.9.1]: docs/releases/v0.9.1.md
[n-0.9.0]: docs/releases/v0.9.0.md
[n-0.8.8]: docs/releases/v0.8.8.md
[n-0.8.5]: docs/releases/v0.8.5.md
[n-0.8.3]: docs/releases/v0.8.3.md

<!-- Version links (to notes files) -->
[0.12.4]: docs/releases/v0.12.4.md
[0.12.3]: docs/releases/v0.12.3.md
[0.12.2]: docs/releases/v0.12.2.md
[0.12.1]: docs/releases/v0.12.1.md
[0.12.0]: docs/releases/v0.12.0.md
[0.11.0]: docs/releases/v0.11.0.md
[0.10.0]: docs/releases/v0.10.0.md
[0.9.1]: docs/releases/v0.9.1.md
[0.9.0]: docs/releases/v0.9.0.md
[0.8.8]: docs/releases/v0.8.8.md
[0.8.5]: docs/releases/v0.8.5.md
[0.8.3]: docs/releases/v0.8.3.md

<!-- GitHub Releases -->
[gh-0.12.4]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.4
[gh-0.12.3]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.3
[gh-0.12.2]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.2
[gh-0.12.1]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.1
[gh-0.12.0]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.12.0
[gh-0.11.0]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.11.0
[gh-0.8.5]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.8.5
[gh-0.8.3]: https://github.com/EffortlessMetrics/perl-lsp/releases/tag/v0.8.3

<!-- Compare ranges -->
[v0.12.3...v0.12.4]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.3...v0.12.4
[v0.12.2...v0.12.3]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.2...v0.12.3
[v0.12.1...v0.12.2]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.1...v0.12.2
[v0.12.0...v0.12.1]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.12.0...v0.12.1
[v0.11.0...v0.12.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.11.0...v0.12.0
[v0.8.5...v0.11.0]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.5...v0.11.0
[v0.8.3...v0.8.5]: https://github.com/EffortlessMetrics/perl-lsp/compare/v0.8.3...v0.8.5

<!-- Channels -->
[vsce]: https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs

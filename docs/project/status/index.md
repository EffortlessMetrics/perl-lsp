# perl-lsp Status Overview

> Human-owned. Edit this file to update release narrative and project state.
> Do **not** add `<!-- BEGIN: -->` markers — generated metrics live in the subsystem files below.

## What's True Right Now

- **Release posture**: GitHub Releases plus the editor channels (VS Code Marketplace and Open VSX) are live on `v0.12.3` as of 2026-04-09, the workspace version line is `v0.12.3`, crates.io intentionally remains on `v0.12.2`, and the active milestone is the `v0.13.0` public alpha announcement
- **Status discipline**: this file is for narrative, subsystem files are for evidence, and `just status-update` plus `just status-check` are the anti-drift workflow
- **LSP server**: `features.toml` is the canonical capability catalog; 53 advertised features at 100% coverage (119/119 total including plumbing protocol methods, DAP handlers, and extension features — count corrected in PR #4107 after the DAP catalog undercount audit, then updated to 117 in PR #4146 and 119 after Stage 2 catalog additions) — computed coverage is generated from it
- **Test infrastructure**: `nix develop -c just ci-gate` is the canonical merge receipt and `cargo xtask ignored-tests` is the tracked-test-debt source
- **Parser stack**: the default parser path is the native recursive-descent stack backed by the Rust lexer and parser-core crates, with three named coverage lanes: Ubuntu system Perl as the compatibility baseline, CPAN top 1000 as the ecosystem-breadth baseline, and the repo-owned corpus as the deterministic regression baseline
- **Refactoring engine**: inline and move-code flows exist; broader refactoring hardening is still roadmap work
- **Safety ratchets**: production baseline currently at `unwrap/expect=0`, panic-family macros (`panic!/todo!/unimplemented!/unreachable!`) = `0`, explicit `unsafe` syntax = `0`
- **Security**: hardening exists for path traversal, command injection, DAP evaluate, and perldoc/perlcritic argument injection

## Subsystem Status

The project tracks metrics across several dimensions that answer different questions. Parser corpus coverage (clean-parse rate) is a floor metric: it measures how broadly the parser handles real-world Perl syntax, not whether the IDE experience is good. LSP/DAP capability coverage is a catalog metric: it measures whether each protocol feature has an implementation wired up, not per-capability correctness or UX quality. End-to-end UX quality is qualitative: validated through manual editor smoke workflows and open-issue burn-down, not a dashboard number. Numbers ratchet forward — they are checked in CI and cannot regress without a deliberate override.

| Subsystem | File | Owner | Updated when |
|-----------|------|-------|-------------|
| LSP coverage & compliance | [lsp.md](lsp.md) | Generator | Every LSP-touching merge |
| Test counts & debt | [tests.md](tests.md) | Generator | Every merge |
| Parser corpus & coverage | [parser.md](parser.md) | Generator | Every parser-touching merge |
<<<<<<< HEAD
| Quality metrics | [quality.md](quality.md) | Generator | Every merge |
| Editor UX planning scaffold | [editor_ux.json](editor_ux.json) | Generator | Every merge |
| Module resolution conformance | [module_resolution.md](module_resolution.md) | Human | After module-resolution changes |
| Release readiness | [release.md](release.md) | Human | Ship readiness changes |

## What's Next

**Now (active milestone: v0.13.0 public alpha announcement)**
- Close out `#3302` demo-asset recording — the main remaining human-owned blocker before the `v0.13.0` public alpha announcement
- Keep the public release split explicit: GitHub Releases, VS Code Marketplace, and Open VSX are on `v0.12.3`, while crates.io remains on `v0.12.2` until the registry window reopens
- Keep the three parser verification lanes explicit and green: `just corpus-sweep-check`, `just cpan-corpus-check`, and `just parser-audit`, with `just common-corpus-check` covering the pinned strict-clean subset
- Keep the top-level README, status docs, and release runbooks aligned with the actual `perllsp` asset line, the `perl-lsp-rs` extension package, and the delayed crates.io surface
- Resume parser, corpus, and semantic hardening while the `v0.13.0` announcement pass stays open

**Next (v0.13.0 public alpha)**
- Keep all three parser corpus lanes current: Ubuntu system Perl, the cached CPAN top 1000 install, and the repo-owned corpus audit
- Fold internal torture and edge-case suites into routine verification receipts
- Publish benchmark and release-readiness receipts for the alpha burndown

**Later (post v0.13.0)**
- DAP preview hardening (deeper live variables/evaluate, shim packaging, cross-editor native receipts)
- Full LSP 3.18 compliance
- Broader distribution packaging

See [ROADMAP.md](../ROADMAP.md) for milestone details.

## Known Constraints

- **Tracked test debt**: see `scripts/ignored-test-count.sh`; feature-gated ignores are by design
- **Docs scope**: `perl-parser` `missing_docs` is ratcheted; workspace-wide enforcement is a separate decision
- **Coverage scope**: the workspace baseline intentionally excludes tests, benches, examples, `archive/`, and embedded tree-sitter crates
- **Coverage gate**: `just coverage-summary` still depends on residual workspace test failures found during the March 17 sweep
- **Index state machine**: verification receipts are captured separately and summarized below

## How to Update

1. Run `just status-update` to regenerate all four subsystem files and the UX receipt
2. Run `just status-update parser` to regenerate only the parser subsystem (post-merge)
3. Run `just status-check` to verify generated sections are current
4. Run `just ci-gate` to verify the repo-level receipt still passes
5. Edit narrative sections (this file, `release.md`) only after the evidence is current

**Historical archives**: see `docs/archive/status_snapshots/` for sprint logs and completion history.

---

*Last Updated: 2026-04-09 (narrative sections only; run `just status-update` to refresh subsystem metrics)*
*Canonical docs: [ROADMAP.md](../ROADMAP.md), [../../features.toml](../../features.toml)*

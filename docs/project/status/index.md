# perl-lsp Status Overview

> Human-owned. Edit this file to update release narrative and project state.
> Do **not** add `<!-- BEGIN: -->` markers — generated metrics live in the subsystem files below.

## What's True Right Now

- **Release posture**: GitHub Releases plus the editor channels (VS Code Marketplace and Open VSX) are live on `v0.12.3` as of 2026-04-09, the workspace version line is `v0.12.3`, crates.io intentionally remains on `v0.12.2`, and the active milestone is the `v0.13.0` public alpha announcement
- **Status discipline**: this file is for narrative, subsystem files are for evidence, and `just status-update` plus `just status-check` are the anti-drift workflow
- **LSP server**: `features.toml` is the canonical capability catalog; 58 user-visible features at 100% coverage (116/116 including plumbing protocol methods and DAP handlers — corrected in PR #4107 after the DAP catalog undercount audit) — computed coverage is generated from it
- **Test infrastructure**: `nix develop -c just ci-gate` is the canonical merge receipt and `cargo xtask ignored-tests` is the tracked-test-debt source
- **Parser stack**: the default parser path is the native recursive-descent stack backed by the Rust lexer and parser-core crates, with three named coverage lanes: Ubuntu system Perl as the compatibility baseline, CPAN top 1000 as the ecosystem-breadth baseline, and the repo-owned corpus as the deterministic regression baseline
- **Refactoring engine**: inline and move-code flows exist; broader refactoring hardening is still roadmap work
- **Safety ratchets**: production baseline currently at `unwrap/expect=0`, panic-family macros (`panic!/todo!/unimplemented!/unreachable!`) = `0`, explicit `unsafe` syntax = `0`
- **Security**: hardening exists for path traversal, command injection, DAP evaluate, and perldoc/perlcritic argument injection

## Subsystem Status

| Subsystem | File | Owner | Updated when |
|-----------|------|-------|-------------|
| LSP coverage & compliance | [lsp.md](lsp.md) | Generator | Every LSP-touching merge |
| Test counts & debt | [tests.md](tests.md) | Generator | Every merge |
| Parser corpus & coverage | [parser.md](parser.md) | Generator | Every parser-touching merge |
| Quality metrics | [quality.md](quality.md) | Generator | Every merge |
| DAP debugger scorecard | [dap.md](dap.md) | Generator | Every DAP-touching merge |
| Release readiness | [release.md](release.md) | Human | Ship readiness changes |
| Workspace & indexing scorecard | [workspace.md](workspace.md) | Generator | Every workspace-touching merge |

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

1. Run `just status-update` to regenerate all four subsystem files
2. Run `just status-update parser` to regenerate only the parser subsystem (post-merge)
3. Run `just status-check` to verify generated sections are current
4. Run `just ci-gate` to verify the repo-level receipt still passes
5. Edit narrative sections (this file, `release.md`) only after the evidence is current

**Historical archives**: see `docs/archive/status_snapshots/` for sprint logs and completion history.

---

*Last Updated: 2026-04-09 (narrative sections only; run `just status-update` to refresh subsystem metrics)*
*Canonical docs: [ROADMAP.md](../ROADMAP.md), [../../features.toml](../../features.toml)*

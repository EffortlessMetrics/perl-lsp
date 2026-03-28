# perl-lsp Status Overview

> Human-owned. Edit this file to update release narrative and project state.
> Do **not** add `<!-- BEGIN: -->` markers — generated metrics live in the subsystem files below.

## What's True Right Now

- **Release posture**: the workspace/release target is `v0.12.0`, the latest published release is `v0.11.0` as verified on 2026-03-28, and the active milestone is `v0.12.0` initial public alpha release prep
- **Status discipline**: this file is for narrative, subsystem files are for evidence, and `just status-update` plus `just status-check` are the anti-drift workflow
- **LSP server**: `features.toml` is the canonical capability catalog; 98 features all at GA maturity — computed coverage is generated from it
- **Test infrastructure**: `nix develop -c just ci-gate` is the canonical merge receipt and `bash scripts/ignored-test-count.sh` is the tracked-test-debt source
- **Parser stack**: the default parser path is the native recursive-descent stack backed by the Rust lexer and parser-core crates
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
| Release readiness | [release.md](release.md) | Human | Ship readiness changes |

## What's Next

**Now (active milestone: v0.12.0 initial public alpha release prep)**
- Raise the CPAN top-1000 full-corpus baseline from `85.4%` (`3717/4355`) to `90%+` clean parses while keeping the strict known-clean manifest at `100%`
- Close repo-corpus coverage gaps (`63/68` NodeKinds currently covered) and retire the remaining parser audit `P2` hang-risk candidate
- Land Moo/Moose/Class::Accessor, `use parent`/`use base`, and export-list disambiguation work needed for public-alpha expectations
- Raise workspace production-code coverage from the new baseline of `44.7%` lines / `46.9%` functions / `42.6%` regions
- Burn down the residual coverage-gate blockers in `perl-parser` control-flow tests and `tree-sitter-perl-rs` parser/heredoc/glob suites
- Keep README, roadmap, status, and release guidance aligned with the split between workspace version and published release

**Next (v0.12.x hardening)**
- Ratchet system-corpus and CPAN baselines as parser coverage improves
- Fold internal torture and edge-case suites into routine verification receipts
- Publish benchmark and release-readiness receipts for the alpha burndown

**Later (post v0.12.x)**
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

*Last Updated: 2026-03-28 (narrative sections only; run `just status-update` to refresh subsystem metrics)*
*Canonical docs: [ROADMAP.md](../ROADMAP.md), [../../features.toml](../../features.toml)*

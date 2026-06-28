# perl-lsp Status Overview

> Human-owned. Edit this file to update release narrative and project state.
> Do **not** add `<!-- BEGIN: -->` markers — generated metrics live in the subsystem files below.

## What's True Right Now

- **Release posture**: `v0.15.0` shipped 2026-05-22 (JSON-RPC type safety + LSP4IJ file-watcher crash fix). The workspace version line is `v0.15.0`, the published crate surface is 31 crates, and `v0.16.0` is the next planned train.
- **Status discipline**: this file is for narrative, subsystem files are for evidence, and `just status-update` plus `just status-check` are the anti-drift workflow
- **LSP server**: `features.toml` is the canonical capability catalog; 60 user-visible features at 100% coverage (125/125 including plumbing protocol methods and DAP handlers) — see generated [lsp.md](lsp.md) for current numbers
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
| HIR lowering coverage | [hir_lowering.md](hir_lowering.md) | Generator | Every HIR lowering merge |
| Compiler fact substrate | [compiler_facts.md](compiler_facts.md) | Human | Compiler-substrate lane changes |
| Real Perl Editor Trust v1 dashboard | [real_perl_editor_trust_v1.md](real_perl_editor_trust_v1.md) | Human | Provider trust, support-claim, or real-workspace receipt changes |
| Provider cutover matrix | [provider_cutover.md](provider_cutover.md) | Human | Provider shadow/live state changes |
| Module resolution conformance | [module_resolution.md](module_resolution.md) | Human | Module-resolution behavior or tracking changes |
| Quality metrics | [quality.md](quality.md) | Generator | Every merge |
| DAP debugger scorecard | [dap.md](dap.md) | Generator | Every DAP-touching merge |
| Release readiness | [release.md](release.md) | Human | Ship readiness changes |
| Workspace & indexing scorecard | [workspace.md](workspace.md) | Generator | Every workspace-touching merge |
| Memory plateau receipts | [memory_plateau.md](memory_plateau.md), [memory_plateau_trends.md](memory_plateau_trends.md) | Human | Memory guardrail, budget, or baseline changes |
| Semantic capability dashboard | [semantic_capability_dashboard.md](semantic_capability_dashboard.md) | Human | Semantic release-readiness changes |
| Semantic UX capability dashboard | [ux_capability_dashboard.md](ux_capability_dashboard.md) | Human | UX surface readiness changes |
| Neovim lean latency profile | [neovim_latency.md](neovim_latency.md) | Human | Lean-profile receipts, smoke scripts, or benchmark evidence |
| Native formatter/critic replacement status | [native_tooling.md](native_tooling.md) | Generator | Native formatter or critic capability changes |
| CI hardening implementation status | [ci_hardening.md](ci_hardening.md) | Human | CI hardening state changes |
| Coverage and RIPR enforcement | [coverage_and_ripr_enforcement.md](coverage_and_ripr_enforcement.md) | Human | Proof-lane policy or transition-exception changes |

## What's Next

**Now (active milestone: v0.16.0 planning — v0.15.0 shipped 2026-05-22)**
- `v0.15.0` is out: JSON-RPC type safety and LSP4IJ file-watcher crash fix shipped
- Keep public-alpha wording consistent: package versions use normal SemVer, but the product posture is not stable/GA
- Keep the three parser verification lanes explicit and green: `just corpus-sweep-check`, `just cpan-corpus-check`, and `just parser-audit`, with `just common-corpus-check` covering the pinned strict-clean subset
- Keep the top-level README, status docs, and release runbooks aligned with the actual `perllsp` asset line, the `perl-lsp-rs` extension package, and the 31-crate published surface
- Keep Homebrew, GitHub release assets, VS Code Marketplace, and Open VSX install receipts explicit in the release closeout

**Next (v0.16.0)**
- Keep all three parser corpus lanes current: Ubuntu system Perl, the cached CPAN top 1000 install, and the repo-owned corpus audit
- Fold internal torture and edge-case suites into routine verification receipts
- Resume parser, corpus, semantic, and DAP hardening after the release-channel receipts are closed
- Track the post-parser semantic build-out through the [compiler-backed LSP roadmap](../COMPILER_BACKED_LSP_ROADMAP.md), keeping generated metrics in subsystem files rather than duplicating them here

**Later**
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

**Historical archives**: see [reference archive](../../reference/archive/) for retained historical docs and completion history.

---

*Last Updated: 2026-06-05 (narrative sections only; run `just status-update` to refresh subsystem metrics)*
*Canonical docs: [ROADMAP.md](../ROADMAP.md), [features.toml](../../../features.toml)*

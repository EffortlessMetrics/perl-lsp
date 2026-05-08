# Compiler Fact Substrate

> Human-owned. Update this page when compiler-substrate lanes change state.
> Generated parser and HIR metric counts belong in their generated status files.

This page tracks the Rust fact layers between parser output and LSP providers.
It is intentionally separate from provider behavior: a fact layer can be
fixture-backed before any live LSP feature consumes it.

## Fact Layer Matrix

| Layer | State | Owner | Evidence | Next proof |
| --- | --- | --- | --- | --- |
| HIR lowering | `fixture-backed` | [#8224](https://github.com/EffortlessMetrics/perl-lsp/issues/8224) | [HIR lowering coverage](hir_lowering.md) | Keep coverage generated as HIR shells expand |
| ScopeGraph / pad facts | `fixture-backed` | [#8193](https://github.com/EffortlessMetrics/perl-lsp/issues/8193) | `crates/perl-parser-core/tests/hir_tests.rs` | Broaden lexical reference and scope-shadow fixtures |
| StashGraph / package facts | `fixture-backed` | [#8194](https://github.com/EffortlessMetrics/perl-lsp/issues/8194) | `crates/perl-parser-core/tests/hir_tests.rs` | Broaden typeglob, inheritance, and dynamic stash fixtures |
| CompileEnvironment facts | `fixture-backed` | [#8206](https://github.com/EffortlessMetrics/perl-lsp/issues/8206) | `crates/perl-parser-core/tests/hir_tests.rs` | Keep configured, lexical, PERL5LIB, and system root provenance explicit |
| Module-resolution candidates | `fixture-backed` | [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242) | `crates/perl-parser-core/tests/hir_tests.rs`; shared include-root builder in `perl-module` | Flow candidate provenance into later resolver and import/export consumers without parser-core environment reads |
| ImportSpec / ExportSet / visible symbols | `semantic-shadowed` | [#8244](https://github.com/EffortlessMetrics/perl-lsp/issues/8244), [#8252](https://github.com/EffortlessMetrics/perl-lsp/issues/8252), [#8253](https://github.com/EffortlessMetrics/perl-lsp/issues/8253) | [Semantic scorecard](semantic_scorecard.md) and [semantic shadow compare](semantic_shadow_compare.md) | Project HIR imports into canonical `ImportSpec` facts, then project HIR/stash exports into canonical `ExportSet` facts |
| Generated-member facts | `fixture-backed` | [#8195](https://github.com/EffortlessMetrics/perl-lsp/issues/8195) | [Semantic scorecard](semantic_scorecard.md) generated-member fixture family | Add adapter registry and Exporter projection in [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245) |
| Compile-time effects | `planned` | [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207) | Roadmap only | Effect records that explain facts and dynamic boundaries |
| Tooling PIR | `planned` | [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196) | Roadmap only | Context-aware PIR lowering fixtures |
| Differential real-Perl oracle | `planned` | [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199) | Roadmap only | Structured conformance receipts; no editor-runtime dependency |

## Boundaries

- `semantic-shadowed` means semantic facts and scorecards exist, but the
  compiler-substrate owner issue still needs to make the surface canonical for
  the Rust compiler path.
- Provider behavior is tracked separately in [provider_cutover.md](provider_cutover.md).
- Runtime module resolution is tracked separately in
  [module_resolution.md](module_resolution.md); HIR module-resolution facts are
  compiler-substrate data and must not spawn Perl or read ambient environment.

## Verification

Use lane-specific checks from the owner issue. Common docs-only checks:

```bash
cargo xtask fmt --check
git diff --check
```

Compiler fact lanes commonly use:

```bash
cargo test -p perl-parser-core --test hir_tests --profile agent --locked -- --nocapture
cargo xtask metrics hir-coverage --check
cargo xtask semantic-scorecard --check
cargo xtask semantic-shadow-compare --check
```

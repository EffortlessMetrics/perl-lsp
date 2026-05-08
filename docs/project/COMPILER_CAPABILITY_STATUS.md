# Compiler Capability Status

This page tracks the Rust compiler-substrate build-out for `perl-lsp`.

The product remains the language server. The compiler substrate is the
load-bearing model that turns parsed Perl into editor facts with provenance,
confidence, and dynamic-boundary behavior.

Do not copy generated parser metrics here. For parser truth, use:

- [Parser status](status/parser.md)
- [Parser accuracy next](status/parser_accuracy_next.md)

## Status Model

Capability states:

| State | Meaning |
| --- | --- |
| `planned` | Issue-owned, no canonical implementation yet |
| `fixture-backed` | Model has focused fixtures, no provider cutover |
| `semantic-shadowed` | Existing semantic facts are scorecarded or shadowed, but canonical compiler-substrate ownership is still being consolidated |
| `shadowed` | Provider impact is measured without changing live behavior |
| `live` | Provider consumes the facts in normal LSP behavior |
| `parked` | Known lane, intentionally not next |

## Capability Table

| Capability | State | Owner issue | Evidence | Next proof |
| --- | --- | --- | --- | --- |
| Parser measurement control plane | `live` | [#4063](https://github.com/EffortlessMetrics/perl-lsp/issues/4063), [#6484](https://github.com/EffortlessMetrics/perl-lsp/issues/6484) | [Parser status](status/parser.md), [parser accuracy next](status/parser_accuracy_next.md) | `cargo xtask metrics parser-accuracy --check`; `cargo xtask update-status --only parser --check` |
| Compiler build-out umbrella | `planned` | [#8191](https://github.com/EffortlessMetrics/perl-lsp/issues/8191) | [Compiler-backed roadmap](COMPILER_BACKED_LSP_ROADMAP.md) | Child checklist stays current |
| Compiler capability status surface | `live` | [#8205](https://github.com/EffortlessMetrics/perl-lsp/issues/8205) | This page | Keep this page current after each compiler-substrate PR |
| HIR lowering | `fixture-backed` | [#8224](https://github.com/EffortlessMetrics/perl-lsp/issues/8224) | [HIR lowering coverage](status/hir_lowering.md) | Keep AST construct coverage generated and current |
| Scope and pad model | `fixture-backed` | [#8193](https://github.com/EffortlessMetrics/perl-lsp/issues/8193) | [Compiler facts](status/compiler_facts.md) | Broaden lexical reference and scope-shadow fixtures; no provider cutover |
| Package and stash model | `fixture-backed` | [#8194](https://github.com/EffortlessMetrics/perl-lsp/issues/8194) | [Compiler facts](status/compiler_facts.md) | Broaden stash/typeglob/inheritance fixtures; no provider cutover |
| Compile environment and module resolution | `fixture-backed` | [#8206](https://github.com/EffortlessMetrics/perl-lsp/issues/8206), [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242) | [Compiler facts](status/compiler_facts.md), [module resolution](status/module_resolution.md) | Keep root provenance explicit as module resolution moves from candidates to consumers |
| Import and export model | `semantic-shadowed` | [#8244](https://github.com/EffortlessMetrics/perl-lsp/issues/8244), [#8252](https://github.com/EffortlessMetrics/perl-lsp/issues/8252), [#8253](https://github.com/EffortlessMetrics/perl-lsp/issues/8253) | [Semantic scorecard](status/semantic_scorecard.md), [semantic shadow compare](status/semantic_shadow_compare.md) | Project HIR imports into `ImportSpec` first, then HIR/stash exports into `ExportSet`; no provider cutover |
| Generated-member facts | `fixture-backed` | [#8195](https://github.com/EffortlessMetrics/perl-lsp/issues/8195) | [Semantic scorecard](status/semantic_scorecard.md) | Adapter registry before broad framework expansion |
| Framework adapter registry | `planned` | [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245) | [Compiler facts](status/compiler_facts.md) | Exporter-family fact projection, no provider cutover |
| Compile-time effect log | `planned` | [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207), [#3394](https://github.com/EffortlessMetrics/perl-lsp/issues/3394) | Roadmap only | Effect records, dynamic-boundary facts, and constant/prototype fixtures |
| Tooling IR / PIR | `planned` | [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196) | Roadmap only | Context-aware PIR lowering fixtures |
| Differential real-Perl oracle | `planned` | [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199) | Roadmap only | Structured agreement receipt; no provider dependency |
| Provider cutover | `planned` | [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) | [Provider cutover](status/provider_cutover.md) | Fact-source tracing, scorecards, and shadow comparison |

## Stop Rules

- Do not cut providers over before the fact layer is fixture-backed and shadowed.
- Do not treat real Perl as the normal editor runtime.
- Do not erase dynamic Perl uncertainty; emit dynamic-boundary facts.
- Do not add retained compiler caches without owner, key, cap, eviction,
  pressure counter, cleanup event, and regression test.
- Do not update generated parser status by hand.

## Common Verification

Use narrow crate checks for implementation PRs. For status-only changes, use:

```bash
cargo xtask fmt --check
git diff --check
```

For parser control-plane freshness, use:

```bash
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
```

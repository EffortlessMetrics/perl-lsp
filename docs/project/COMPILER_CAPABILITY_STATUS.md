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
| `partial live` | At least one narrowly scoped provider path consumes proven facts with fallback, while broader cutover remains gated |
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
| Compile environment and module resolution | `fixture-backed` | [#8206](https://github.com/EffortlessMetrics/perl-lsp/issues/8206), [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242), [#8270](https://github.com/EffortlessMetrics/perl-lsp/issues/8270), [#8275](https://github.com/EffortlessMetrics/perl-lsp/issues/8275), [#8280](https://github.com/EffortlessMetrics/perl-lsp/issues/8280) | [Compiler facts](status/compiler_facts.md), [module resolution](status/module_resolution.md), [#8284](https://github.com/EffortlessMetrics/perl-lsp/pull/8284) | Current fixture-backed compile-environment lane is complete; downstream compile-time effects are tracked in [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207), and provider use remains gated by [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Import and export model | `fixture-backed` | [#8244](https://github.com/EffortlessMetrics/perl-lsp/issues/8244), [#8252](https://github.com/EffortlessMetrics/perl-lsp/issues/8252), [#8253](https://github.com/EffortlessMetrics/perl-lsp/issues/8253), [#8264](https://github.com/EffortlessMetrics/perl-lsp/issues/8264) | `crates/perl-parser-core/tests/hir_tests.rs`, [#8256](https://github.com/EffortlessMetrics/perl-lsp/pull/8256), [#8260](https://github.com/EffortlessMetrics/perl-lsp/pull/8260), [#8262](https://github.com/EffortlessMetrics/perl-lsp/pull/8262), [#8267](https://github.com/EffortlessMetrics/perl-lsp/pull/8267), [Semantic scorecard](status/semantic_scorecard.md), [semantic shadow compare](status/semantic_shadow_compare.md) | Canonical visible-symbol proof is complete; provider consumption remains gated by [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Generated-member facts | `fixture-backed` | [#8195](https://github.com/EffortlessMetrics/perl-lsp/issues/8195) | [Semantic scorecard](status/semantic_scorecard.md), [#8287](https://github.com/EffortlessMetrics/perl-lsp/pull/8287) | Broader Moo/Moose/Class::Tiny/Object::Pad adapters remain parked until the next issue-owned slice |
| Framework adapter registry | `fixture-backed` | [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245) | `crates/perl-parser-core/tests/hir_tests.rs`, [#8287](https://github.com/EffortlessMetrics/perl-lsp/pull/8287) | Exporter-family registry slice is complete; provider use remains gated by [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Compile-time effect log | `fixture-backed` | [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207), [#3394](https://github.com/EffortlessMetrics/perl-lsp/issues/3394), [#8293](https://github.com/EffortlessMetrics/perl-lsp/issues/8293), [#8294](https://github.com/EffortlessMetrics/perl-lsp/issues/8294) | `crates/perl-parser-core/tests/hir_tests.rs`, [#8291](https://github.com/EffortlessMetrics/perl-lsp/pull/8291), [#8297](https://github.com/EffortlessMetrics/perl-lsp/pull/8297), [#8300](https://github.com/EffortlessMetrics/perl-lsp/pull/8300) | HIR effect-log, symbolic-ref boundary, and selected differential-oracle proof slices are complete; provider use remains gated by [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Tooling IR / PIR | `planned` | [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196) | Roadmap only | Context-aware PIR lowering fixtures |
| Differential real-Perl oracle | `planned` | [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199) | Roadmap only | Structured agreement receipt; no provider dependency |
| Provider cutover | `partial live` | [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) | [Provider cutover](status/provider_cutover.md), [semantic shadow compare](status/semantic_shadow_compare.md), [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305), [#8319](https://github.com/EffortlessMetrics/perl-lsp/issues/8319), [#8327](https://github.com/EffortlessMetrics/perl-lsp/issues/8327), [#8342](https://github.com/EffortlessMetrics/perl-lsp/pull/8342), [#8344](https://github.com/EffortlessMetrics/perl-lsp/pull/8344), [#8349](https://github.com/EffortlessMetrics/perl-lsp/pull/8349), [#8351](https://github.com/EffortlessMetrics/perl-lsp/pull/8351), [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353), [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359) | Semantic-token provider fact-source proof remains insufficient data; broader live cutover stays gated |

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

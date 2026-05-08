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
| `shadowed` | Provider impact is measured without changing live behavior |
| `live` | Provider consumes the facts in normal LSP behavior |
| `parked` | Known lane, intentionally not next |

## Capability Table

| Capability | State | Owner issue | Next proof |
| --- | --- | --- | --- |
| Parser measurement control plane | `live` | [#4063](https://github.com/EffortlessMetrics/perl-lsp/issues/4063), [#6484](https://github.com/EffortlessMetrics/perl-lsp/issues/6484) | `cargo xtask metrics parser-accuracy --check`; `cargo xtask update-status --only parser --check` |
| Compiler build-out umbrella | `planned` | [#8191](https://github.com/EffortlessMetrics/perl-lsp/issues/8191) | Child checklist stays current |
| Compiler capability status surface | `live` | [#8205](https://github.com/EffortlessMetrics/perl-lsp/issues/8205) | Keep this page current after each compiler-substrate PR |
| HIR lowering | `fixture-backed` | [#8224](https://github.com/EffortlessMetrics/perl-lsp/issues/8224) | [HIR lowering coverage](status/hir_lowering.md) tracks AST construct coverage; no provider cutover |
| Scope and pad model | `fixture-backed initial` | [#8193](https://github.com/EffortlessMetrics/perl-lsp/issues/8193) | HIR `ScopeGraph` records scope frames, storage-classed bindings, lexical references, and shadowing; no provider cutover |
| Package and stash model | `fixture-backed initial` | [#8194](https://github.com/EffortlessMetrics/perl-lsp/issues/8194) | HIR `StashGraph` records package stashes, glob slots, inheritance edges, and dynamic stash boundaries; no provider cutover |
| Compile environment and module resolution | `fixture-backed initial` | [#8206](https://github.com/EffortlessMetrics/perl-lsp/issues/8206) | HIR `CompileEnvironment` records use/no/require directives, pragma effects, use-lib roots, module requests, phase blocks, and dynamic boundaries; module path resolution remains planned |
| Import and export model | `planned` | [#3413](https://github.com/EffortlessMetrics/perl-lsp/issues/3413), [#3414](https://github.com/EffortlessMetrics/perl-lsp/issues/3414), [#3415](https://github.com/EffortlessMetrics/perl-lsp/issues/3415), [#3416](https://github.com/EffortlessMetrics/perl-lsp/issues/3416), [#3474](https://github.com/EffortlessMetrics/perl-lsp/issues/3474), [#7485](https://github.com/EffortlessMetrics/perl-lsp/issues/7485), [#7492](https://github.com/EffortlessMetrics/perl-lsp/issues/7492) | Canonical `ImportSpec` / `ExportSet` facts and visible-symbol proof |
| Framework adapters | `planned` | [#8195](https://github.com/EffortlessMetrics/perl-lsp/issues/8195) | Generated-member facts with framework provenance |
| Compile-time effect log | `planned` | [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207), [#3394](https://github.com/EffortlessMetrics/perl-lsp/issues/3394) | Effect records, dynamic-boundary facts, and constant/prototype fixtures |
| Tooling IR / PIR | `planned` | [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196) | Context-aware PIR lowering fixtures |
| Differential real-Perl oracle | `planned` | [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199) | Structured agreement receipt; no provider dependency |
| Provider cutover | `planned` | [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) | Fact-source tracing, scorecards, and shadow comparison |

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

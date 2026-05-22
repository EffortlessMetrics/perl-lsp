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
| Module-resolution candidates | `fixture-backed` | [#8242](https://github.com/EffortlessMetrics/perl-lsp/issues/8242), [#8270](https://github.com/EffortlessMetrics/perl-lsp/issues/8270), [#8275](https://github.com/EffortlessMetrics/perl-lsp/issues/8275), [#8280](https://github.com/EffortlessMetrics/perl-lsp/issues/8280) | `crates/perl-parser-core/tests/hir_tests.rs`; shared include-root builder in `perl-module`; [module resolution status](module_resolution.md) | Current compile-environment/module-resolution fixture lane is complete; keep facts available for downstream compile-time effects and provider proof |
| ImportSpec / ExportSet / visible symbols | `fixture-backed` | [#8244](https://github.com/EffortlessMetrics/perl-lsp/issues/8244), [#8252](https://github.com/EffortlessMetrics/perl-lsp/issues/8252), [#8253](https://github.com/EffortlessMetrics/perl-lsp/issues/8253), [#8264](https://github.com/EffortlessMetrics/perl-lsp/issues/8264) | `crates/perl-parser-core/tests/hir_tests.rs`, [#8256](https://github.com/EffortlessMetrics/perl-lsp/pull/8256), [#8260](https://github.com/EffortlessMetrics/perl-lsp/pull/8260), [#8262](https://github.com/EffortlessMetrics/perl-lsp/pull/8262), [#8267](https://github.com/EffortlessMetrics/perl-lsp/pull/8267), [Semantic scorecard](semantic_scorecard.md), and [semantic shadow compare](semantic_shadow_compare.md) | Feed provider fact-source tracing and cutover gates in [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Generated-member facts | `fixture-backed` | [#8195](https://github.com/EffortlessMetrics/perl-lsp/issues/8195), [#8245](https://github.com/EffortlessMetrics/perl-lsp/issues/8245) | [Semantic scorecard](semantic_scorecard.md) generated-member fixture family; [#8287](https://github.com/EffortlessMetrics/perl-lsp/pull/8287) | Broader Moo/Moose/Class::Tiny/Object::Pad adapters remain issue-owned and parked |
| Compile-time effects | `fixture-backed` | [#8207](https://github.com/EffortlessMetrics/perl-lsp/issues/8207), [#3394](https://github.com/EffortlessMetrics/perl-lsp/issues/3394), [#8293](https://github.com/EffortlessMetrics/perl-lsp/issues/8293), [#8294](https://github.com/EffortlessMetrics/perl-lsp/issues/8294) | `crates/perl-parser-core/tests/hir_tests.rs`, [#8291](https://github.com/EffortlessMetrics/perl-lsp/pull/8291), [#8297](https://github.com/EffortlessMetrics/perl-lsp/pull/8297), [#8300](https://github.com/EffortlessMetrics/perl-lsp/pull/8300) | Keep effect records available for downstream tooling IR and provider proof; provider use remains gated by [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) |
| Prototype table facts | `fixture-backed` | [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) | `crates/perl-parser-core/tests/hir_tests.rs` | Keep named subroutine prototype content and source ranges available as compiler substrate; no provider, diagnostic, parser-bucket, PIR, or support-tier promotion follows from this row |
| Bareword classifier facts | `fixture-backed` | [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197) | `crates/perl-parser-core/tests/hir_tests.rs` | Keep source-backed syntactic bareword roles available for downstream diagnostic/provider proof; no PL109 suppression, provider behavior, parser-bucket, PIR, determinism, or support-tier promotion follows from this row |
| Tooling PIR | `planned` | [#8196](https://github.com/EffortlessMetrics/perl-lsp/issues/8196) | Roadmap only | Context-aware PIR lowering fixtures |
| Differential real-Perl oracle | `planned / manifest-declared / receipt-schema-declared` | [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199), [#8294](https://github.com/EffortlessMetrics/perl-lsp/issues/8294) | [PLSP-SPEC-0027](../../specs/PLSP-SPEC-0027-differential-real-perl-oracle.md); [oracle fixture manifest](../../../crates/perl-corpus/fixtures/differential_oracle/manifest.json); [oracle receipt schema](../../../schemas/oracle_receipt.v1.schema.json); selected compile-effect oracle proof in [#8300](https://github.com/EffortlessMetrics/perl-lsp/pull/8300) | Broader conformance receipts remain planned; real Perl stays a comparison oracle, not an editor-runtime dependency |

## Boundaries

- `semantic-shadowed` means semantic facts and scorecards exist, but the
  compiler-substrate owner issue still needs to make the surface canonical for
  the Rust compiler path.
- `fixture-backed` import/export facts mean HIR projections emit canonical
  `ImportSpec` and `ExportSet` values, and [#8267](https://github.com/EffortlessMetrics/perl-lsp/pull/8267)
  proves `visible_symbols_at` over those facts. Provider behavior remains
  separate until provider-impact proof and [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197)
  cutover gates are satisfied.
- Provider behavior is tracked separately in [provider_cutover.md](provider_cutover.md).
- Runtime module resolution is tracked separately in
  [module_resolution.md](module_resolution.md); HIR module-resolution facts are
  compiler-substrate data and must not spawn Perl or read ambient environment.
- Real Perl is useful for differential proof, but the durable fact substrate is
  the Rust-native compiler path that feeds editor, formatter, linter,
  refactoring, and determinism workflows.
- The differential oracle contract is manifest-declared and receipt-schema
  declared planning only. The checked-in manifest declares fixture identities
  and environment boundaries, and the receipt schema locks the future receipt
  shape, but neither adds an oracle runner, executes Perl, probes workspaces,
  moves parser/corpus buckets, promotes support tiers, or changes provider
  behavior.

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

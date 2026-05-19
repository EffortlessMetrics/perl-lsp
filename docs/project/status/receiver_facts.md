# Receiver Facts Status

> Human-owned. This page tracks implementation state for
> [PLSP-SPEC-0005](../../specs/PLSP-SPEC-0005-receiver-expression-facts.md).
> It does not generate metrics, broaden completion behavior, or replace
> provider cutover receipts.

Receiver facts are the semantic substrate for evidence-backed method receiver
behavior. Fact availability alone is not provider cutover. Completion, hover,
goto, diagnostics, or refactors may consume receiver facts only after their
provider-specific fallback and confidence receipts satisfy
[PLSP-SPEC-0002](../../specs/PLSP-SPEC-0002-provider-confidence-receipts.md).

Current implementation plan:
[RECEIVER_FACTS_IMPLEMENTATION_PLAN.md](../RECEIVER_FACTS_IMPLEMENTATION_PLAN.md).
Current provider cutover state:
[provider_cutover.md](provider_cutover.md).

## Claim Boundary

Current receiver-facts work is semantic substrate only.

It may claim:

- rich fact model and type-environment storage where tests prove it
- receiver extraction over existing `TypeFact` and `ShapeFact` evidence
- dynamic-key boundaries for receiver extraction
- no completion candidate behavior change

It may not claim:

- receiver-backed completion cutover
- hover, goto, diagnostics, or refactor behavior changes
- support-tier promotion
- expression-level fact inference from Perl source declarations and
  assignments until `infer_expr_fact` or equivalent fixtures prove it

Facts-only PRs must keep this wording in their claim boundary:

```text
semantic substrate only
no completion candidate behavior change
no support-tier promotion
```

## Status Rows

| Area | Status | Current proof | Boundary / next step |
| --- | --- | --- | --- |
| `fact_model` | `landed` | `crates/perl-semantic-analyzer/src/analysis/type_facts.rs`; `crates/perl-semantic-analyzer/tests/type_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | `TypeFact`, `ShapeFact`, `HashShape`, `ArrayShape`, `ObjectShape`, `TypeEvidence`, and `DynamicBoundary` exist as substrate. |
| `type_environment_fact_map` | `landed` | `TypeEnvironment::set_variable_fact`, `get_variable_fact`, `get_fact_at`; stale fact clearing and parent lookup tests in `type_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Existing `PerlType` callers keep erased compatibility; source-level expression inference is separate. |
| `static_package_receiver` | `landed` | `receiver_facts` module test `static_constructor_receiver_records_package`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Static package receivers can produce high-confidence constructor evidence. |
| `object_variable_receiver` | `landed` | `receiver_facts` module tests for `$self` and `$object`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Exact package requires a supplied type-environment fact. Unknown object variables stay low confidence. |
| `hash_slot_receiver` | `partial` | `receiver_facts` module test `hash_slot_receiver_uses_known_slot_fact`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Works when `TypeEnvironment` already contains a `HashShape`; hash literal and assignment expression inference is still missing. |
| `hashref_slot_receiver` | `partial` | `receiver_facts` module test `hashref_slot_receiver_preserves_hashref_kind`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Works when the base fact already has a hash shape; `$hashref->{key}` fact production from source declarations is still missing. |
| `array_index_receiver` | `partial` | `receiver_facts` module tests for static and dynamic array indexes; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Static indexes can use existing `ArrayShape` facts; dynamic indexes remain non-exact. |
| `dynamic_key_boundary` | `landed` | `receiver_facts` module test `dynamic_hash_key_marks_dynamic_boundary`; `TypeFact::dynamic` test in `type_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Proven for receiver extraction. Broader expression and provider boundary receipts remain pending. |
| `expression_inference` | `missing` | No `TypeInferenceEngine::infer_expr_fact` API on current `master` | Next semantic slice should infer facts from literals, declarations, assignments, constructor calls, and static hash/hashref slots. |
| `receiver_fact_api` | `landed` | `crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | API extracts facts from existing AST and supplied environment facts; method-call chains remain unknown until explicit rules land. |
| `completion_cutover` | `blocked` | No completion provider usage of `ReceiverFact` on current `master` | Blocked by facts-only fixtures, expression inference, provider fallback proof, and receiver confidence receipts. |

## Provider Cutover Dashboard

```text
receiver_fact_completion_cutover:
  facts_substrate: partial
  completion_consumes_fact: no
  fallback_proven: pending
  dynamic_boundary_proven: partial receiver-extraction only
  support_claim_allowed: no
```

## Next Implementation Steps

1. Add expression-level fact inference for literals, assignments,
   declarations, constructor calls, and hash/hashref slot reads.
2. Add facts-only fixtures that prove source-derived `%hash` and `$hashref`
   receiver facts without changing provider output.
3. Add provider confidence receipts for exact, fallback, and dynamic receiver
   cases.
4. Cut completion over only after the facts-only and provider fallback receipts
   pass.

## Proof Commands

Use these checks for semantic receiver-facts implementation PRs:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe test -p perl-semantic-analyzer --test type_facts --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe test -p perl-semantic-analyzer --lib receiver_facts --profile agent --locked -- --nocapture
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe clippy -p perl-semantic-analyzer --profile agent --locked -- -D warnings -A missing_docs
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask fmt
git diff --check
```

Docs-only status updates may run:

```bash
git diff --check
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask ci-hygiene check-doc-paths docs/project/status
```

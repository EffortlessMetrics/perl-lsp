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

Receiver facts are semantic substrate by default. A separate completion cutover
receipt now consumes one narrow source-backed receiver class, but facts-only PRs
remain substrate-only until provider-specific proof promotes another class.

It may claim:

- rich fact model and type-environment storage where tests prove it
- receiver extraction over existing `TypeFact` and `ShapeFact` evidence
- source-derived constructor, plain-hash, hashref, and static bless-field facts
  where tests prove them
- source-derived framework accessor-return facts where tests prove package-like
  `isa` declarations
- source-derived method-return facts where tests prove a static constructor
  return, a lexical local initialized from a static constructor or accessor
  chain, or a lexical local assigned from one
- dynamic-key boundaries for receiver extraction
- no completion candidate behavior change unless the PR is explicitly a
  provider cutover receipt tied to [PLSP-SPEC-0007](../../specs/PLSP-SPEC-0007-receiver-fact-completion.md)

It may not claim:

- receiver-backed completion cutover beyond the separately proved narrow
  source-backed pilot
- hover, goto, diagnostics, or refactor behavior changes
- support-tier promotion
- broader conditional method-return facts until focused fixtures
  prove those source shapes

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
| `type_environment_fact_map` | `landed` | `TypeEnvironment::set_variable_fact`, `get_variable_fact`, `get_fact_at`; stale fact clearing and parent lookup tests in `type_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Existing `PerlType` callers keep erased compatibility while source-level expression inference stores richer facts for proven shapes. |
| `static_package_receiver` | `landed` | `receiver_facts` module test `static_constructor_receiver_records_package`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Static package receivers can produce high-confidence constructor evidence. |
| `object_variable_receiver` | `landed` | `receiver_facts` module tests for `$self` and `$object`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Exact package requires a supplied type-environment fact. Unknown object variables stay low confidence. |
| `hash_slot_receiver` | `partial` | `receiver_facts` module test `hash_slot_receiver_uses_known_slot_fact`; `receiver_expression_facts` tests for plain hash literals and slot assignment | Works when `TypeEnvironment` contains a `HashShape`; source-derived plain `%hash` literals and `$hash{key}` assignments can now populate that shape. Broader chained method-return source inference remains pending. |
| `hashref_slot_receiver` | `partial` | `receiver_facts` module test `hashref_slot_receiver_preserves_hashref_kind`; `receiver_expression_facts` tests for hashref literals and slot assignment | Works when the base fact already has a hash shape; source-derived `$hashref->{key}` facts are proven for hashref literals and slot assignment. Broader chained method-return source inference remains pending. |
| `bless_field_receiver` | `partial` | `receiver_expression_facts` tests for static bless fields and dynamic bless class fallback; `receiver_facts` module test `object_field_receiver_preserves_fallback` | Static `bless { field => Package->new }, 'Class'` populates medium-confidence object-field facts for `$self->{field}`. Dynamic bless class names fail closed. This is semantic substrate only and does not broaden completion. |
| `accessor_return_receiver` | `partial` | `receiver_expression_facts` tests for Moo/Moose package-like `isa` accessor returns, source-derived `$self` constructor assignment plus framework accessor returns, mismatched `$self` package fallback, and dynamic `isa` fallback | Static framework declarations such as `has db => (isa => 'MyApp::DB')` populate medium-confidence object-shape facts for `$service->db`, and source-derived `$self = MyApp::Service->new` evidence can route the same accessor-return shape for `$self->db` only when the receiver package matches the framework declaration. The erased type remains `Any`, dynamic/non-package `isa` values and mismatched receiver packages fail closed, and this is semantic substrate only with no completion cutover. |
| `method_return_receiver` | `partial` | `receiver_expression_facts` tests for direct static constructor method returns, constructor-to-framework-accessor method-return chains, lexical local constructor/accessor-chain variable returns, lexical assignment evidence, and dynamic constructor/accessor fallback | A method body with a single `return MyApp::DB->new`, implicit `MyApp::DB->new` expression, `return MyApp::Container->new->db` when `db` is a source-backed framework accessor, `my $db = MyApp::DB->new; return $db;`, `my $db = MyApp::Container->new->db; return $db;`, or corresponding lexical assignment shapes can populate a medium-confidence object-shape fact for `$service->db`. The erased type remains `Any`; dynamic constructor/accessor receivers, dynamic reassignments, conditional reassignments, and unscoped bare assignments fail closed; and this is semantic substrate only with no completion cutover. |
| `array_index_receiver` | `partial` | `receiver_facts` module tests for static and dynamic array indexes; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | Static indexes can use existing `ArrayShape` facts; dynamic indexes remain non-exact. |
| `dynamic_key_boundary` | `landed` | `receiver_facts` module test `dynamic_hash_key_marks_dynamic_boundary`; `TypeFact::dynamic` test in `type_facts.rs`; `receiver_expression_facts` dynamic plain-hash-key test; completion provider test `dynamic_hash_key_receiver_preserves_imported_fallback` | Proven for receiver extraction, plain hash expression facts, and the first completion-provider boundary receipt. Additional provider boundary receipts remain pending for other receiver forms. |
| `expression_inference` | `partial` | `crates/perl-semantic-analyzer/tests/receiver_expression_facts.rs`; `TypeInferenceEngine::infer_expr_fact` | Constructor calls, source-derived `$self` constructor assignment, plain hash literals, plain hash slot assignment, hashref literals, hashref slot assignment, static plain/hashref slot reads, static bless fields, framework accessor returns, direct static constructor method returns, static constructor-to-framework-accessor method-return chains, lexical local constructor/accessor-chain variable method returns, dynamic plain hash keys, dynamic bless classes, dynamic/non-package accessor `isa` values, dynamic method-return constructors/accessor chains, dynamic method-return variable reassignments, conditional reassignments, and unscoped method-return assignments are facts-only substrate. Broader conditional method-return facts remain pending. |
| `receiver_fact_api` | `landed` | `crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs`; PR [#9468](https://github.com/EffortlessMetrics/perl-lsp/pull/9468) | API extracts facts from existing AST and supplied environment facts; broader method-call chains remain unknown until explicit rules land. |
| `completion_cutover` | `narrow-pilot` | Completion provider tests `source_backed_hash_slot_receiver_uses_exact_completion_pilot`, `dynamic_hash_key_receiver_preserves_imported_fallback`, `medium_confidence_accessor_return_receiver_preserves_imported_fallback`, and `medium_confidence_method_return_receiver_preserves_imported_fallback`; [RealReceiver real-workspace quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_46_receiver_real_workspace_quality.rs); [RealReceiver method/accessor fallback receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_47_receiver_method_accessor_fallback.rs); [RealReceiver bless confidence receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_48_receiver_bless_confidence.rs); [RealReceiver array-index fallback receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_49_receiver_array_index_fallback.rs); [RealReceiver self/this quality receipt](../../../crates/perl-lsp-ux-tests/tests/ux_scenario_50_receiver_self_this_quality.rs); PR [#9502](https://github.com/EffortlessMetrics/perl-lsp/pull/9502) | Only fresh high-confidence source-backed receiver facts may authorize exact receiver completion. The RealReceiver quality receipt records current project-shaped behavior without promotion: constructor-assignment and plain hash-slot completion act source-backed, static package completion acts as exact high-confidence syntax evidence, while hashref-slot, dynamic-key, and unknown receiver probes remain low-confidence fallback. The method/accessor fallback receipt records accessor-return, method-return, local accessor-chain method-return, dynamic local accessor-chain method-return, and conditional local-reassignment method-return receiver chains preserving low-confidence fallback and tier-6 sorting instead of exact source-backed receiver detail. The bless confidence receipt records literal `bless` as medium-confidence labeled evidence and dynamic `bless` as legacy workspace fallback without exact receiver evidence. The array-index fallback receipt records static and dynamic array-index receiver chains preserving low-confidence fallback and tier-6 sorting instead of exact source-backed receiver detail. The self/this quality receipt records `$self->` and `$this->` local methods as ordinary local method candidates, inherited workspace methods as exact high-confidence self/this syntax evidence, and nearest-shadow behavior preferring the local method without broader receiver promotion. Unknown, dynamic, generated/no-source, stale, low-confidence, medium-confidence accessor-return, medium-confidence method-return, medium-confidence local accessor-chain method-return, dynamic local accessor-chain method-return, conditional local-reassignment method-return, medium-confidence literal-bless, and unpromoted array-index receiver shapes stay fallback, shadowed, labeled, or blocked until separate provider receipts promote one class. |

## Provider Cutover Dashboard

```text
receiver_fact_completion_cutover:
  facts_substrate: partial
  completion_consumes_fact: narrow source-backed pilot
source_backed_proven: constructor assignment plus plain hash-slot receiver pilot
exact_syntax_proven: static package receiver plus inherited self/this workspace receiver detail
  fallback_proven: dynamic hash key, hashref slot, generated/no-source framework method, method/accessor/local-accessor-chain/dynamic-local-accessor-chain/conditional-local-reassignment receiver chains, dynamic bless, and array-index receivers preserve fallback boundaries
  dynamic_boundary_proven: receiver extraction plus completion-provider boundary
  support_claim_allowed: partial-live-with-fallback only
```

## Next Implementation Steps

1. Add more real-workspace and additional receiver-form provider confidence
   receipts for exact, fallback, and dynamic receiver cases beyond the
RealReceiver constructor/static-package/hash-slot/hashref/generated-no-source-framework-method/dynamic/unknown,
   method/accessor/local-accessor-chain/dynamic-local-accessor-chain/
   conditional-local-reassignment fallback, bless confidence, and array-index
   fallback, and self/this current-package receipts.
2. Broaden completion only after facts-only and provider fallback receipts pass
   for the new receiver class.

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

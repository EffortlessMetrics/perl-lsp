# PLSP-SPEC-0005: Receiver expression facts

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: none yet
Linked plan: [Receiver facts implementation plan](../project/RECEIVER_FACTS_IMPLEMENTATION_PLAN.md)
Status impact: provider cutover, semantic scorecard, semantic capability dashboard,
UX capability dashboard

## Current implementation status

This spec is accepted as the receiver-fact substrate contract. The initial fact
model, type-environment fact map, receiver fact API, static package and object
receiver facts, plain-hash slot inference, dynamic-key boundary, and the first
completion consumer are implemented or partially implemented as tracked in
[receiver_facts.md](../project/status/receiver_facts.md).

Current implementation state and next source-shape work live in the status doc
and receiver-facts implementation plan, not in this spec. Hashref literal and
slot-assignment inference are fixture-backed facts-only substrate. Static
`bless { field => Package->new }, 'Class'` field inference is fixture-backed as
medium-confidence substrate. Framework accessor-return facts for package-like
`isa` declarations are fixture-backed as medium-confidence substrate with erased
type compatibility preserved. Direct static-constructor method-return facts are
fixture-backed as medium-confidence substrate with erased type compatibility
preserved, and static constructor-to-framework-accessor method-return chains are
fixture-backed as medium-confidence substrate with erased type compatibility
preserved. Receiver facts expose explicit fallback state so provider consumers
can distinguish exact, fallback-preserving, and blocked receiver evidence.
Broader local-variable chained and conditional method-return facts remain
bounded by their own future fixtures and provider receipts.

## Contract

`perl-lsp` must infer method receiver facts from parsed expressions before a
provider claims exact method completion, hover, definition, or other
method-receiver behavior.

The receiver-facts lane is a semantic join between the existing AST, the
semantic type environment, workspace method resolution, and provider confidence
receipts. It must not require new AST node variants for hash, hashref, array, or
arrayref access before the first implementation slice. The parser already emits
postfix access as `NodeKind::Binary` operators, so this spec requires consumers
to recognize these existing shapes:

| Perl expression shape | Required AST contract |
| --- | --- |
| `$h{k}` | `NodeKind::Binary { op: "{}", left, right }` |
| `$h->{k}` | `NodeKind::Binary { op: "->{}", left, right }` |
| `$a[0]` | `NodeKind::Binary { op: "[]", left, right }` |
| `$a->[0]` | `NodeKind::Binary { op: "->[]", left, right }` |
| `$obj->method` | `NodeKind::MethodCall { object, method, args }` |

The supported pipeline is:

```text
parse receiver expression
→ infer receiver fact
→ resolve package/method set
→ rank by confidence
→ prove with fixtures
```

## Fact Model Requirements

Add a fact layer on top of `PerlType`; do not replace `PerlType` in the first
receiver-facts wave.

The first slice must introduce a `TypeFact` wrapper that carries:

- erased coarse type, via the existing `PerlType`
- confidence
- evidence
- optional dynamic-boundary reason
- optional structural shape

The first shape set must include:

- `HashShape`, with static slots and an optional fallback value fact
- `ArrayShape`, with indexed slots and an optional element fact
- `ObjectShape`, with package and field facts; object fields may be scaffolded
  before the bless/Object::Pad/Moo/Moose slices populate them

The initial evidence vocabulary must cover:

- literal inference
- variable initializers and assignments
- hash/hashref slots
- constructor calls
- bless literals
- Moo/Moose `isa`
- Object::Pad fields
- workspace-symbol evidence
- accessor-return evidence
- method-return evidence
- heuristic evidence

The initial dynamic-boundary vocabulary must cover:

- dynamic hash keys
- dynamic bless class names
- dynamic method names
- runtime imports
- unknown receivers

Existing APIs that only expose `PerlType` may keep returning erased facts until
their provider-specific cutover satisfies this spec and
[PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md).

## Type Environment Requirements

`TypeEnvironment` must keep the current variable-to-`PerlType` behavior and add
a parallel variable-to-`TypeFact` map.

Required operations:

- set a variable fact and update the erased variable type at the same time
- fetch a borrowed variable fact through parent scopes
- fetch an owned variable fact through parent scopes
- update a static hash slot after assignment
- preserve existing `get_type_at`/coarse-type behavior for compatibility when
  that API exists or is added

The fact map is not a replacement for lexical scoping. It must follow the same
scope lookup rules as variable types.

## Expression Inference Requirements

`TypeInferenceEngine` must expose expression-level fact inference before
completion consumes it. The required first API shape is:

```rust
pub fn infer_expr_fact(
    &mut self,
    node: &Node,
    env: &mut TypeEnvironment,
) -> TypeFact
```

The first implementation must handle these AST variants or fail closed:

| AST node | Required behavior |
| --- | --- |
| `Variable` | return a stored variable fact when present; otherwise return the erased variable type as a low-confidence fact when available |
| `HashLiteral` | infer static slot facts and a fallback value fact |
| `ArrayLiteral` | infer indexed and element facts where practical |
| `Assignment` with `op == "="` | update variable or static hash-slot facts and return the right-hand fact |
| `VariableDeclaration` | store initializer facts for lexical variables; hash declarations must preserve hash shape |
| `MethodCall` | infer constructor and method-return facts where rules exist |
| `FunctionCall` | infer `bless` and other explicit function-return rules where rules exist |
| `Binary` with `op == "{}"` | resolve static plain-hash slots; dynamic keys must fail closed |
| `Binary` with `op == "->{}"` | resolve static hashref/object-field slots; dynamic keys must fail closed |
| `Binary` with `op == "[]"` or `op == "->[]"` | resolve static array slots where known; dynamic indexes must fail closed or return fallback element facts only |

Dynamic boundaries must not be promoted to exact packages. A dynamic receiver may
still participate in legacy fallback completion only when the fallback is labeled
as low confidence or unknown, not as an exact semantic receiver.
Expression inference that returns a receiver-shaped fact must set
`fallback_state` consistently with the confidence, dynamic-boundary, freshness,
and package evidence it returns.

## Static Hash and Hashref Requirements

The first exact receiver milestone is static hash and hashref access.

### Hash literal assignment

For:

```perl
my %services = (
    db => MyApp::DB->new,
);
$services{db}->connect;
```

receiver inference for `$services{db}` must produce:

- package: `MyApp::DB`
- confidence: high
- evidence containing a hash slot for `services`/`db`
- evidence containing a constructor call for `MyApp::DB`

### Hash slot assignment

For:

```perl
my %services;
$services{db} = MyApp::DB->new;
$services{db}->connect;
```

receiver inference must produce the same package and evidence class as the
literal assignment case, with assignment evidence included.

### Hashref literal assignment

For:

```perl
my $services = {
    db => MyApp::DB->new,
};
$services->{db}->connect;
```

receiver inference for `$services->{db}` must produce package `MyApp::DB` with
hashref-slot evidence.

### Dynamic key boundary

For:

```perl
my %services = (db => MyApp::DB->new);
$services{$name}->connect;
```

receiver inference must not produce package `MyApp::DB` as an exact receiver.
It must record `DynamicHashKey` or an equivalent dynamic-boundary fact.

## Constructor and Method-Return Requirements

The first method-return rule is constructor recognition:

```perl
MyApp::Thing->new
```

When the receiver is a static package expression and the method name is `new`,
the method call must infer an object fact for that package with high confidence
and constructor-call evidence.

Later method-return rules may extend the same fact path for:

- Moo/Moose accessors from `has ... isa => 'Package'`
- direct method bodies that return a static constructor
- Object::Pad reader/accessor field methods
- `DBI->connect(...)` returning `DBI::db`
- `$dbh->prepare(...)` returning `DBI::st`
- simple builder/accessor return chains

These later rules must not be implemented as completion-only heuristics once the
fact layer exists; completion, hover, definition, and signature-help surfaces
should share the same semantic evidence where practical.

## Receiver Fact Requirements

Before completion is cut over, the semantic analyzer must expose a receiver-fact
contract such as:

```rust
pub struct ReceiverFact {
    pub kind: ReceiverKind,
    pub package: Option<String>,
    pub shape: Option<ShapeFact>,
    pub confidence: Confidence,
    pub evidence: Vec<TypeEvidence>,
    pub freshness: ReceiverFactFreshness,
    pub dynamic_boundary: Option<DynamicBoundary>,
    pub source_range: Option<(usize, usize)>,
    pub fallback_state: ReceiverFallbackState,
}
```

`ReceiverKind` must distinguish at least:

- static package
- variable
- hash slot
- hashref slot
- array index
- dynamic key
- method call receiver chains
- unknown receiver

For a method call, `receiver_fact_for_method_call` must infer the object
expression and set `package` only for exact object/reference-to-object facts.
Unknown, dynamic, and low-confidence non-object receivers must not set an exact
package.

Every receiver fact must carry `fallback_state`. The fallback state is part of
the semantic contract, not a provider-local display field:

```rust
pub enum ReceiverFallbackState {
    Exact,
    Fallback,
    Blocked,
}
```

Provider consumers may treat `Exact` as receiver-scoped evidence only when all
other provider guards also pass. `Fallback` means the provider must preserve its
legacy fallback path and may surface the receiver fact only as labeled evidence.
`Blocked` means the receiver fact cannot authorize completion, navigation, or
edit-producing behavior.

## Receiver Fallback Semantics

Receiver exactness is intentionally narrower than package availability.
`package: Some(_)` alone is not enough to authorize exact provider behavior.

| Receiver shape | Required fallback state |
| --- | --- |
| Exact source-backed receiver | May be `Exact` only when the fact is fresh, high-confidence, source-backed, and has no dynamic boundary. |
| Union receiver | Must be `Fallback` unless every branch resolves to the same package or a class-specific proof resolves the ambiguity. |
| Dynamic receiver | Must be `Fallback` or `Blocked`; it must never be promoted to exact behavior. |
| Unknown receiver | Must be `Fallback` and low confidence unless a later fact class proves a stronger shape. |
| Generated receiver | Must be labeled and fallback-preserving, or blocked; it must never be silently exact. |
| Low- or medium-confidence receiver | Must be `Fallback` until a provider-specific promotion receipt proves the class. |
| Stale receiver | Must be `Fallback` or `Blocked`; it must not authorize edits. |

Generated, virtual, framework-derived, or no-source receiver evidence may help
explain completion and hover output after the relevant provider receipt lands.
It must not claim an exact generated method-body location, must not suppress
fallback, and must not authorize rename or safe-delete unless a separate
class-specific refactor proof exists.

## Provider Cutover Requirements

Completion may consume receiver facts only after facts-only tests pass. The
provider must keep existing text-pattern receiver classification as fallback
until exact receiver-fact behavior is fixture-backed.

The handoff shape is:

1. Provider asks semantic analyzer for an optional `ReceiverFact` at the method
   arrow context.
2. Workspace method completion receives the optional receiver fact.
3. Receiver evidence is derived from the fact when present.
4. Exact receiver behavior may use `ReceiverFact.package` only when
   `fallback_state == Exact` and provider-specific confidence guards pass.
5. Legacy text-pattern classification remains available when no semantic
   receiver fact is available or when the semantic receiver fact is `Fallback`.
6. `Blocked` receiver facts must not authorize completion, navigation, or
   edit-producing behavior.
7. Unknown or dynamic receivers fall back according to provider confidence rules
   and must not be labeled as exact semantic receivers.

Completion details for exact receiver facts should include receiver kind,
confidence, and relevant evidence once the provider handoff lands.

## Acceptance Fixtures

Facts-only tests must land before UI behavior changes. The dedicated semantic
analyzer test surface should cover these scenarios:

| Scenario | Required result |
| --- | --- |
| `%hash` literal slot | `$services{db}` resolves to `MyApp::DB`, high confidence, hash-slot and constructor evidence |
| `%hash` slot assignment | `$services{db}` resolves to `MyApp::DB`, high confidence, assignment/hash-slot and constructor evidence |
| hashref literal slot | `$services->{db}` resolves to `MyApp::DB` |
| dynamic key | package is `None`; dynamic boundary is `DynamicHashKey` |
| bless object field | `$self->{db}` resolves to `MyApp::DB`, medium confidence after bless-field slice |
| Moo/Moose accessor | `$self->db` or another source-backed object accessor resolves to `MyApp::DB` after framework-accessor slice, as medium-confidence substrate until provider receipts promote it |
| direct method return | `$self->db` or another source-backed object method resolves to `MyApp::DB` after a direct `return MyApp::DB->new` slice, as medium-confidence substrate until provider receipts promote it |

After facts-only tests pass, LSP completion tests must prove:

- `$services{db}->` offers methods from `MyApp::DB`
- completion detail labels the receiver as hash-slot derived
- completion detail labels confidence
- `$services{$name}->` does not claim an exact `MyApp::DB` receiver

Receiver-fact tests must also prove fallback posture:

- fresh high-confidence source-backed receiver facts use `Exact`
- medium-confidence accessor-return and method-return facts use `Fallback`
- union receivers use `Fallback` unless ambiguity is explicitly resolved
- dynamic hash keys and dynamic array indexes use `Fallback` or `Blocked`
- unknown receivers use `Fallback`
- generated or framework-derived receiver facts remain labeled and
  fallback-preserving until provider-specific proof promotes the class

## Proof Commands

Use the narrowest safe checks for each PR slice:

```bash
./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked
./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked
./scripts/cargo-safe check --all-targets -p perl-lsp-rs-core --profile agent --locked
./scripts/cargo-safe xtask fmt
./scripts/cargo-safe clippy -p perl-semantic-analyzer --profile agent --locked -- -D warnings -A missing_docs
./scripts/cargo-safe clippy -p perl-lsp-rs-core --profile agent --locked -- -D warnings -A missing_docs
just agent-pr-fast
```

A docs-only PR for this spec may use formatting and link checks when available,
plus `git diff --check`.

## Non-goals

- Do not add dedicated AST node variants for hash/hashref/array/arrayref access
  in the first receiver-facts wave.
- Do not rewrite the parser to support receiver facts.
- Do not remove legacy completion receiver classification until semantic facts
  are fixture-backed and provider fallback behavior is proven.
- Do not claim C3 MRO correctness as part of the first hash-slot milestone.
- Do not broaden completion to noisy all-workspace fallback as a substitute for
  exact receiver facts.
- Do not use dynamic keys, dynamic bless classes, or runtime imports as exact
  receiver evidence.

## Claim Boundaries

A PR may claim exact hash-slot receiver support only when static hash literal or
slot-assignment fixtures prove package, confidence, and evidence.

A PR may claim hashref support only when `$hashref->{key}` fixtures prove the
same behavior independently of plain `%hash` access.

A PR may claim object-field support only when bless/Object::Pad/framework tests
prove the specific object model slice.

A PR may claim provider completion support only when LSP completion fixtures
prove labels and exact/non-exact behavior. Facts-only PRs must describe the work
as semantic substrate, not user-visible completion behavior.

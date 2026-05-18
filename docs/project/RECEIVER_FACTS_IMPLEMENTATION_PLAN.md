# Receiver Facts Implementation Plan

Status: proposed
Owner: perl-lsp maintainers
Spec: [PLSP-SPEC-0005: Receiver expression facts](../specs/PLSP-SPEC-0005-receiver-expression-facts.md)
Related proposal: [PLSP-PROP-0001: Real Perl editor trust](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)

## Objective

Build one narrow receiver pipeline instead of a broad type-engine rewrite:

```text
parse receiver expression
→ infer receiver fact
→ resolve package/method set
→ rank by confidence
→ prove with fixtures
```

The implementation should recognize the AST shapes that already exist for
postfix access and method calls. It should add semantic facts on top of the AST,
not a parser rewrite and not new AST node variants in the first wave.

## Design Choice Locked for the First Wave

Do not add new AST variants for hash or array access yet.

The first wave consumes the existing parser output:

| Perl surface | AST shape to consume |
| --- | --- |
| `$h{k}` | `NodeKind::Binary { op: "{}", left, right }` |
| `$h->{k}` | `NodeKind::Binary { op: "->{}", left, right }` |
| `$a[0]` | `NodeKind::Binary { op: "[]", left, right }` |
| `$a->[0]` | `NodeKind::Binary { op: "->[]", left, right }` |
| `$obj->method` | `NodeKind::MethodCall { object, method, args }` |

Dedicated `HashAccess`, `HashRefAccess`, `ArrayAccess`, or `ArrayRefAccess`
nodes may be considered later for readability, but they are not a prerequisite
for receiver facts.

## Work Breakdown

### PR 1 — Facts model and environment

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_facts.rs` | Add fact model types. |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Extend `TypeEnvironment` with variable facts. |
| `crates/perl-semantic-analyzer/src/analysis/mod.rs` or equivalent module root | Export the new facts module if required by crate structure. |

**Deliverables**

- `TypeFact`
- `ShapeFact`
- `HashShape`
- `ArrayShape`
- `ObjectShape`
- `TypeEvidence`
- `DynamicBoundary`
- `TypeFact::erased_type()` compatibility helper
- `TypeEnvironment::set_variable_fact`
- `TypeEnvironment::get_variable_fact`
- `TypeEnvironment::get_fact_at`
- No completion behavior change

**Notes**

`set_variable_fact` must update both the erased `variables` map and the richer
`variable_facts` map. Parent-scope lookup for facts should mirror parent-scope
lookup for existing variable types.

**Suggested validation**

```bash
./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked
./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked
./scripts/cargo-safe xtask fmt
```

### PR 2 — Expression fact inference for constructors and plain hashes

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Add `infer_expr_fact`, hash literal inference, constructor inference, declaration and assignment fact updates. |
| `crates/perl-semantic-analyzer/tests/receiver_facts.rs` | Add facts-only regression tests for plain hashes and dynamic keys. |

**Deliverables**

- `HashLiteral` to `HashShape`
- `Class->new` to `PerlType::Object(Class)` fact
- `%hash` declaration stores hash shape
- `$hash{key}` resolves static slot facts
- `$hash{$dynamic}` fails closed with `DynamicHashKey`
- `$hash{key} = Constructor->new` updates the hash slot fact

**Implementation sequence**

1. Add `infer_expr_fact` as a public semantic API.
2. Add `static_hash_key(node)` for identifiers, strings, and numbers.
3. Add `infer_hash_literal_fact` with static slots and fallback value.
4. Add `unify_type_facts` for fallback values; keep it simple and conservative.
5. Add `static_package_expr(node)` and constructor recognition for `new`.
6. Teach variable declarations to store initializer facts.
7. Teach assignments to update variables and plain hash slots.
8. Add `infer_plain_hash_slot` for `op == "{}"`.
9. Add facts-only tests for literal slot, slot assignment, and dynamic key.

**Suggested validation**

```bash
./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked receiver_facts
./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked
./scripts/cargo-safe clippy -p perl-semantic-analyzer --profile agent --locked -- -D warnings -A missing_docs
```

### PR 3 — Hashref slots and object-field scaffolding

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Add hashref-slot lookup and object-field lookup scaffolding. |
| `crates/perl-semantic-analyzer/tests/receiver_facts.rs` | Add hashref facts-only test. |

**Deliverables**

- `$hashref->{key}` shape lookup
- `$self->{key}` lookup path that can consume `ObjectShape` later
- Dynamic hashref keys fail closed with `DynamicHashKey`
- Tests for `$services->{db}->connect`

**Implementation sequence**

1. Add `infer_hashref_slot` for `op == "->{}"`.
2. Reuse `static_hash_key` for key extraction.
3. Infer the left side as an expression fact rather than requiring a variable.
4. Return a hash slot when the base fact has `ShapeFact::Hash`.
5. Return an object field when the base fact has `ShapeFact::Object`.
6. Add dynamic-key regression coverage.

### PR 4 — Receiver facts API

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs` | Add receiver fact types and conversion from AST nodes. |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Add `receiver_fact_for_method_call` or equivalent delegating API. |

**Deliverables**

- `ReceiverFact`
- `ReceiverExpr`
- `receiver_fact_for_method_call`
- `receiver_expr_from_node`
- Package extraction for object and reference-to-object facts
- Facts-only tests that call the receiver API directly

**Implementation sequence**

1. Add receiver expression enum variants for static package, variable, hash slot,
   hashref slot, method call, and unknown.
2. Convert AST object nodes into receiver expression descriptions.
3. Call `infer_expr_fact` for the method-call object.
4. Set `ReceiverFact.package` only for exact object facts.
5. Preserve evidence and dynamic-boundary information in the embedded `TypeFact`.

### PR 5 — Completion handoff without deleting fallback

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/request/dispatch.rs` | Compute or pass optional receiver fact at method-arrow completion. |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs` | Accept optional receiver facts and derive receiver evidence from them. |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs` or equivalent completion test surface | Add exact hash-slot completion tests. |

**Deliverables**

- `add_workspace_method_completions` accepts optional receiver facts
- Existing text-pattern classifier remains as fallback
- Exact hash-slot receiver completions work for static hash facts
- Completion detail includes receiver kind and confidence once evidence is wired
- Dynamic key test proves no exact receiver claim

**Implementation sequence**

1. Add provider helper to find the receiver expression at the current arrow
   completion context.
2. Parse or reuse AST for the source region as narrowly as existing provider
   structure allows.
3. Ask semantic analyzer for `ReceiverFact`.
4. Pass `Option<&ReceiverFact>` into workspace method completions.
5. Map `ReceiverFact` to `ReceiverEvidence`.
6. If no fact exists, run legacy text-pattern classification.
7. Use `ReceiverFact.package` for exact package method lookup.
8. Preserve unknown fallback behavior, but label dynamic/unknown as non-exact.

### PR 6 — Bless object fields

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Recognize bless literals and store package object-field facts. |
| `crates/perl-semantic-analyzer/tests/receiver_facts.rs` | Add bless field facts-only test. |

**Deliverables**

- `bless { field => Constructor->new }, $class` captures object fields when the
  package/class context is static enough
- `$self->{field}` resolves inside package methods
- Confidence is medium unless the constructor/package pattern is exceptionally
  direct
- Dynamic bless class names fail closed

**Implementation sequence**

1. Add package-context plumbing needed by the inference call.
2. Detect `FunctionCall { name: "bless", args }`.
3. If the first arg is a hash literal, infer its hash shape.
4. If the class argument is a static package or clean `$class` invocant pattern,
   convert hash slots into `ObjectShape` fields.
5. Store package object facts in the engine or environment boundary selected by
   the implementation.
6. Recognize `$self` as invocant in methods where that is already represented.
7. Resolve `$self->{field}` through stored object-field facts.
8. Add dynamic bless negative tests.

### PR 7 — Framework accessor returns

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Add method-return facts from class models. |
| `crates/perl-semantic-analyzer/src/analysis/class_model.rs` or existing class-model access surface | Expose accessor metadata if not already accessible. |
| `crates/perl-semantic-analyzer/tests/receiver_facts.rs` | Add Moo/Moose and Object::Pad accessor tests. |

**Deliverables**

- Moo/Moose `has db => (isa => 'MyApp::DB')` makes `$self->db` an object fact
- Object::Pad reader/accessor/mutator metadata can feed method-return facts
- `$self->db->connect` can resolve against `MyApp::DB`
- Evidence distinguishes `MooseIsa` and `ObjectPadField`

**Implementation sequence**

1. Add `infer_accessor_return(package, method, class_models)` helper.
2. Parse simple package names from `isa` values conservatively.
3. Map accessor metadata to object facts with medium confidence.
4. Wire method-call inference to invoke accessor return rules when receiver
   package is known.
5. Add tests for positive accessor returns and dynamic/unknown accessors.

### PR 8 — Shared method-return rules

**Primary files**

| Path | Change |
| --- | --- |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Add common method-return rule enum and DBI rules. |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs` | Remove or delegate DBI completion-only receiver heuristics once semantic facts are proven. |
| Semantic and completion tests | Prove DBI and chained method-return behavior. |

**Deliverables**

- `MethodReturnRule::ConstructorNew`
- `MethodReturnRule::MooseAccessor`
- `MethodReturnRule::ObjectPadAccessor`
- `MethodReturnRule::DBIConnect`
- `MethodReturnRule::DBIPrepare`
- `DBI->connect(...)` infers `DBI::db`
- `$dbh->prepare(...)` infers `DBI::st` when `$dbh` is a DBI database handle fact
- Completion consumes these as semantic facts instead of owning duplicate rules

## Minimal Acceptance Matrix

| Case | Example | Required outcome |
| --- | --- | --- |
| Hash literal slot | `my %services = (db => MyApp::DB->new); $services{db}->connect;` | exact `MyApp::DB`, high confidence, hash-slot + constructor evidence |
| Hash slot assignment | `$services{db} = MyApp::DB->new; $services{db}->connect;` | exact `MyApp::DB`, high confidence, assignment + constructor evidence |
| Hashref literal slot | `my $services = { db => MyApp::DB->new }; $services->{db}->connect;` | exact `MyApp::DB` |
| Dynamic key | `$services{$name}->connect;` | no exact package; dynamic hash-key boundary |
| Bless object field | `bless { db => MyApp::DB->new }, $class; $self->{db}->connect;` | exact `MyApp::DB`, medium confidence when package context is clean |
| Moo/Moose accessor | `has db => (is => 'ro', isa => 'MyApp::DB'); $self->db->connect;` | `$self->db` fact is `Object(MyApp::DB)` |
| Dynamic bless | `bless {}, $class; $self->{db}->connect;` | no exact object-field package without field evidence |

## Test Plan

### Facts-only tests

Add `crates/perl-semantic-analyzer/tests/receiver_facts.rs` before provider UI
changes.

Initial facts-only tests:

1. `%hash` literal slot resolves to package and evidence.
2. `%hash` slot assignment updates shape and resolves later receiver.
3. `$hashref->{key}` resolves to package.
4. Dynamic key returns `DynamicHashKey` and no exact package.
5. Bless field resolves `$self->{field}` after bless-field slice.
6. Moo/Moose accessor resolves `$self->accessor` after accessor slice.

### Completion tests

After facts-only tests pass, add completion tests in the completion-provider test
surface.

Required assertions:

- completion at `$services{db}->` contains a `MyApp::DB` method such as
  `connect`
- completion detail includes receiver kind, such as hash slot
- completion detail includes confidence
- completion at `$services{$name}->` does not claim an exact `MyApp::DB`
  receiver; fallback, if present, is low-confidence or unknown

## Validation Commands

Use targeted checks per PR slice. Avoid workspace-wide direct Cargo commands in
normal agent work.

| Gate | Command | Use when |
| --- | --- | --- |
| Semantic tests | `./scripts/cargo-safe test -p perl-semantic-analyzer --profile agent --locked` | semantic fact slices |
| Semantic check | `./scripts/cargo-safe check --all-targets -p perl-semantic-analyzer --profile agent --locked` | semantic fact slices |
| Semantic clippy | `./scripts/cargo-safe clippy -p perl-semantic-analyzer --profile agent --locked -- -D warnings -A missing_docs` | semantic fact slices before PR |
| Completion tests | `./scripts/cargo-safe test -p perl-lsp-rs-core --profile agent --locked` | provider handoff slices |
| Completion check | `./scripts/cargo-safe check --all-targets -p perl-lsp-rs-core --profile agent --locked` | provider handoff slices |
| Completion clippy | `./scripts/cargo-safe clippy -p perl-lsp-rs-core --profile agent --locked -- -D warnings -A missing_docs` | provider handoff slices before PR |
| Formatting | `./scripts/cargo-safe xtask fmt` | every code slice |
| Local PR gate | `just agent-pr-fast` | before review when feasible |
| Storage guard | `./scripts/storage-doctor` | after substantial local builds |

## Rollback Plan

- PR 1 is substrate-only. Revert removes fact types and environment additions;
  no user-visible behavior should change.
- PRs 2 through 4 must remain semantic-only. They may add fact inference,
  storage, and tests, but they must not change provider output. If provider
  wiring becomes necessary, split it into PR 5 or a later handoff PR. Rollback
  removes only the corresponding semantic inference capability and tests.
- PR 5 is the first user-visible completion slice. Rollback should restore the
  old `add_workspace_method_completions` signature and rely solely on legacy
  receiver classification.
- PRs 6 through 8 add more fact sources. Each should be independently guarded by
  tests and should be revertible without removing the core hash-slot facts.

## Documentation and Status Discipline

- Specs define behavior contracts and claim limits.
- This plan defines implementation order.
- Generated status files remain truth sources for current metrics and should not
  be hand-edited for this lane.
- Completion cutover claims must cite provider receipts or tests rather than
  this plan alone.

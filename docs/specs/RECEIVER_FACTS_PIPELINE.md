# Receiver Facts Pipeline Specification

## Status

Planning specification. This document defines the narrow implementation path for turning parsed Perl receiver expressions into evidence-backed method-completion receivers. It intentionally does **not** require new AST node variants.

## Problem

Method completion currently has useful pieces in separate layers:

1. the parser already preserves receiver-expression structure;
2. the semantic analyzer has coarse `PerlType` inference;
3. workspace completion can collect package methods; and
4. completion has text-pattern receiver heuristics.

The missing seam is the join between those pieces. Receiver completion should not start by asking, “can we improve the whole type engine?” It should implement one pipeline:

```text
parse receiver expression
→ infer receiver fact
→ resolve package/method set
→ rank by confidence
→ prove with fixtures
```

The first target is static structure such as:

```perl
my %services = (
    db => MyApp::DB->new,
);

$services{db}->connect;
```

The expected receiver package is `MyApp::DB`, with high-confidence evidence that the receiver came from the `services` hash slot `db` and the value came from a `MyApp::DB->new` constructor call.

## Non-goals

- Do not add dedicated `HashAccess`, `HashRefAccess`, `ArrayAccess`, or `ArrayRefAccess` AST variants in the first cut.
- Do not replace `PerlType`; richer facts wrap it and expose an erased `PerlType` for compatibility.
- Do not delete the existing completion text-pattern heuristics until facts-backed fixtures prove replacement behavior.
- Do not implement exact C3 MRO as a blocker for receiver facts; method collection can keep its current traversal until the receiver seam works.
- Do not turn dynamic Perl into false precision. Dynamic keys, dynamic class expressions, and runtime imports must fail closed or remain explicitly low-confidence.

## Existing AST contract

The parser already emits enough structure for the receiver pipeline. The facts layer must consume these existing forms:

```rust
NodeKind::Binary { op: "{}",   left, right }   // $h{k}
NodeKind::Binary { op: "->{}", left, right }   // $h->{k}
NodeKind::Binary { op: "[]",   left, right }   // $a[0]
NodeKind::Binary { op: "->[]", left, right }   // $a->[0]
NodeKind::MethodCall { object, method, args }  // $obj->method
```

The implementation should match the operator string on `NodeKind::Binary` rather than changing parser output.

## Facts model

Add a semantic facts layer in `crates/perl-semantic-analyzer`:

```text
crates/perl-semantic-analyzer/src/analysis/type_facts.rs
crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs
```

### Type facts

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TypeFact {
    pub ty: PerlType,
    pub confidence: Confidence,
    pub evidence: Vec<TypeEvidence>,
    pub dynamic_boundary: Option<DynamicBoundary>,
    pub shape: Option<ShapeFact>,
}

impl TypeFact {
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }
}
```

`TypeFact` keeps the existing coarse `PerlType` while adding confidence, evidence, dynamic-boundary state, and optional structural shape.

### Shape facts

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFact {
    Hash(HashShape),
    Array(ArrayShape),
    Object(ObjectShape),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HashShape {
    pub slots: BTreeMap<String, TypeFact>,
    pub fallback_value: Option<Box<TypeFact>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayShape {
    pub indexed: BTreeMap<usize, TypeFact>,
    pub element: Option<Box<TypeFact>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectShape {
    pub package: String,
    pub fields: BTreeMap<String, TypeFact>,
}
```

### Evidence

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TypeEvidence {
    Literal,
    VariableInitializer { name: String },
    Assignment { name: String },
    HashSlot { hash: String, key: String },
    HashRefSlot { base: String, key: String },
    ConstructorCall { package: String },
    BlessLiteral { package: String },
    MooseIsa { attr: String, isa: String },
    ObjectPadField { field: String },
    WorkspaceSymbol { package: String },
    Heuristic { reason: String },
}
```

### Dynamic boundaries

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicBoundary {
    DynamicHashKey,
    DynamicBlessClass,
    DynamicMethodName,
    RuntimeImport,
    UnknownReceiver,
}
```

Dynamic boundaries are not errors. They are evidence that precise inference must stop.

## Type environment extension

Extend `TypeEnvironment` with facts parallel to the existing variable type map:

```rust
pub struct TypeEnvironment {
    variables: HashMap<String, PerlType>,
    variable_facts: HashMap<String, TypeFact>,
    subroutines: HashMap<String, PerlType>,
    parent: Option<Box<TypeEnvironment>>,
}
```

Required APIs:

```rust
impl TypeEnvironment {
    pub fn set_variable_fact(&mut self, name: String, fact: TypeFact) {
        self.variables.insert(name.clone(), fact.erased_type());
        self.variable_facts.insert(name, fact);
    }

    pub fn get_variable_fact(&self, name: &str) -> Option<&TypeFact> {
        self.variable_facts
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_variable_fact(name)))
    }

    pub fn get_fact_at(&self, name: &str) -> Option<TypeFact> {
        self.get_variable_fact(name).cloned()
    }
}
```

`get_type_at` / existing type lookup behavior must keep returning erased `PerlType` values for current callers.

## Expression fact inference

Add expression-level fact inference to `TypeInferenceEngine`:

```rust
impl TypeInferenceEngine {
    pub fn infer_expr_fact(
        &mut self,
        node: &Node,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        // match node.kind
    }
}
```

The first implementation must handle:

```rust
match &node.kind {
    NodeKind::Variable { sigil, name } => { /* env lookup or sigil default */ }
    NodeKind::HashLiteral { pairs } => { /* hash shape */ }
    NodeKind::ArrayLiteral { elements } => { /* array shape */ }
    NodeKind::Assignment { lhs, rhs, op } if op == "=" => { /* update facts */ }
    NodeKind::VariableDeclaration { variable, initializer, .. } => { /* store facts */ }
    NodeKind::MethodCall { object, method, args } => { /* constructor/method returns */ }
    NodeKind::FunctionCall { name, args } => { /* bless and later rules */ }
    NodeKind::Binary { op, left, right } if op == "{}" => { /* plain hash slot */ }
    NodeKind::Binary { op, left, right } if op == "->{}" => { /* hashref/object field slot */ }
    NodeKind::Binary { op, left, right } if op == "[]" || op == "->[]" => { /* array lookup */ }
    _ => TypeFact::any_low_confidence(/* reason */),
}
```

## Static key extraction

Only static keys produce precise slot facts:

```rust
fn static_hash_key(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::String { value, .. } => Some(value.clone()),
        NodeKind::Number { value } => Some(value.clone()),
        _ => None,
    }
}
```

When the key is not static, return a fact with `dynamic_boundary: Some(DynamicBoundary::DynamicHashKey)` and no package.

## Hash shape inference

### Hash literal assigned to `%hash`

For:

```perl
my %services = (
    db    => MyApp::DB->new,
    cache => MyApp::Cache->new,
);
```

infer:

- `PerlType::Hash { key: Scalar(String), value: Union([...]) }`;
- `ShapeFact::Hash` with slots `db` and `cache`;
- fallback value equal to the union of literal value facts;
- high confidence;
- evidence containing `Literal`, `HashSlot`, and constructor evidence for each slot value.

### Slot assignments

For:

```perl
my %services;
$services{db} = MyApp::DB->new;
```

`infer_expr_fact` for the assignment must update the stored shape for `services`:

```rust
fn plain_hash_slot(node: &Node) -> Option<(String, String)> {
    let NodeKind::Binary { op, left, right } = &node.kind else {
        return None;
    };
    if op != "{}" {
        return None;
    }
    let NodeKind::Variable { sigil, name } = &left.kind else {
        return None;
    };
    if sigil != "$" {
        return None;
    }
    let key = static_hash_key(right)?;
    Some((name.clone(), key))
}
```

`TypeEnvironment::update_hash_slot` should create an unknown hash fact when the hash exists only as a bare declaration, then update `ShapeFact::Hash.slots` and the fallback value.

## Receiver slot lookup

### Plain hash slot

For `NodeKind::Binary { op: "{}", left, right }`:

1. extract a static key;
2. require the left side to be a scalar variable representing the hash slot expression (`$services{db}` uses `$services` in the AST);
3. look up `env.get_variable_fact(name)`;
4. return an exact slot fact when present;
5. otherwise return fallback value if present;
6. otherwise return unknown.

A dynamic key must return `DynamicHashKey`, not the fallback value.

### Hashref slot

For `NodeKind::Binary { op: "->{}", left, right }`:

1. extract a static key;
2. infer the base expression fact;
3. if the base has `ShapeFact::Hash`, return its slot/fallback;
4. if the base has `ShapeFact::Object`, return its field fact;
5. otherwise return unknown.

This supports `$services->{db}` immediately and scaffolds `$self->{db}` for object-field inference.

## Constructor recognition

`TypeInferenceEngine::infer_method_call_fact` must recognize `Class->new` as an object fact:

```rust
fn infer_method_call_fact(
    &mut self,
    object: &Node,
    method: &str,
    args: &[Node],
    env: &mut TypeEnvironment,
) -> TypeFact {
    if method == "new" {
        if let Some(package) = static_package_expr(object) {
            return TypeFact {
                ty: PerlType::Object(package.clone()),
                confidence: Confidence::High,
                evidence: vec![TypeEvidence::ConstructorCall { package }],
                dynamic_boundary: None,
                shape: None,
            };
        }
    }

    TypeFact::unknown()
}
```

`static_package_expr` must only accept static package syntax. Variables and dynamic expressions are not package facts.

## Receiver facts API

Add `receiver_facts.rs` as the contract between semantic analysis and LSP completion:

```rust
pub struct ReceiverFact {
    pub receiver: ReceiverExpr,
    pub fact: TypeFact,
    pub package: Option<String>,
}

pub enum ReceiverExpr {
    StaticPackage(String),
    Variable(String),
    HashSlot { base: String, key: String },
    HashRefSlot { base: Box<ReceiverExpr>, key: String },
    MethodCall { receiver: Box<ReceiverExpr>, method: String },
    Unknown,
}
```

Required engine API:

```rust
impl TypeInferenceEngine {
    pub fn receiver_fact_for_method_call(
        &mut self,
        object: &Node,
        env: &mut TypeEnvironment,
    ) -> ReceiverFact {
        let fact = self.infer_expr_fact(object, env);
        let package = match &fact.ty {
            PerlType::Object(pkg) => Some(pkg.clone()),
            PerlType::Reference(inner) => match inner.as_ref() {
                PerlType::Object(pkg) => Some(pkg.clone()),
                _ => None,
            },
            _ => None,
        };

        ReceiverFact {
            receiver: receiver_expr_from_node(object),
            fact,
            package,
        }
    }
}
```

## Completion handoff

The completion provider should eventually pass a semantic `ReceiverFact` to workspace method completion instead of making workspace completion own type semantics.

Current shape:

```rust
workspace::add_workspace_method_completions(
    completions,
    context,
    source,
    provider.type_engine.as_ref(),
    &provider.workspace_index,
    &provider.used_modules,
);
```

Target shape:

```rust
let receiver_fact = provider.receiver_fact_at(source, position);

workspace::add_workspace_method_completions(
    completions,
    context,
    source,
    receiver_fact.as_ref(),
    &provider.workspace_index,
    &provider.used_modules,
);
```

Inside workspace completion, facts-backed evidence should be preferred and text-pattern classification should remain fallback:

```rust
let evidence = receiver_fact
    .map(ReceiverEvidence::from_receiver_fact)
    .unwrap_or_else(|| classify_text_pattern_receiver(context, source));
```

Unknown receiver fallback may remain, but exact receiver completions must require a package from semantic facts or from the existing high/medium-confidence fallback evidence.

## Object fields from `bless`

Support object field facts after hash and hashref slots are stable.

For:

```perl
package MyApp::Service;
sub new {
    my $class = shift;
    bless { db => MyApp::DB->new }, $class;
}
sub run {
    my $self = shift;
    $self->{db}->connect;
}
```

Implementation requirements:

1. When inferring `FunctionCall { name: "bless", args }`, detect a first-argument hash literal.
2. Resolve the class only when it is a static package string or a clean `$class` constructor idiom tied to the current package.
3. Convert the hash shape into an `ObjectShape` for that package.
4. Store package object fields for later method-body lookup.
5. Resolve `$self->{field}` inside package methods through the stored `ObjectShape`.
6. Mark clean `bless` object-field facts medium confidence unless stronger evidence is added later.
7. Dynamic bless classes must return `DynamicBoundary::DynamicBlessClass` and no exact package.

## Framework accessor returns

After receiver facts and object fields, use the existing class-model data for generated accessors.

### Moo/Moose

For:

```perl
has db => (
    is  => 'ro',
    isa => 'MyApp::DB',
);

$self->db->connect;
```

infer `$self->db` as `Object("MyApp::DB")` when the current class model has an attribute whose accessor name matches `db` and whose `isa` is a simple package type. Evidence should be `TypeEvidence::MooseIsa { attr, isa }` and confidence should start at medium.

### Object::Pad

Use `FieldInfo.reader`, `writer`, `accessor`, `mutator`, and initializer/default evidence to infer field accessor returns. Evidence should be `TypeEvidence::ObjectPadField { field }`.

## Method-return rules

Introduce method-return facts after receiver facts land:

```rust
pub enum MethodReturnRule {
    ConstructorNew,
    MooseAccessor,
    ObjectPadAccessor,
    DBIConnect,
    DBIPrepare,
}
```

Initial known cases:

- `MyApp::Thing->new(...)` returns `MyApp::Thing`.
- `DBI->connect(...)` returns `DBI::db`.
- `$dbh->prepare(...)` returns `DBI::st` when `$dbh` is known as `DBI::db`.
- `$self->db` returns `MyApp::DB` when class-model accessor metadata proves it.

DBI heuristics should move from completion-only logic into facts so completion, hover, and future signature help share the same evidence.

## Method resolution and ranking

The first cut may keep current method collection traversal. Once exact receiver facts exist, add:

```rust
resolve_method_candidates(package, method_prefix, mro_mode)
```

Use class-model MRO data to distinguish DFS and C3 where available. Ranking should prefer higher-confidence exact receiver facts over low-confidence fallbacks and should annotate completion details with the receiver source and confidence.

## Test plan

### Facts-only tests

Add:

```text
crates/perl-semantic-analyzer/tests/receiver_facts.rs
```

Required fixtures:

1. `%hash` literal slot:

   ```perl
   package MyApp::DB;
   sub connect {}

   package main;
   my %services = (
       db => MyApp::DB->new,
   );

   $services{db}->connect;
   ```

   Assert `receiver.package == Some("MyApp::DB")`, high confidence, `HashSlot { hash: "services", key: "db" }`, and `ConstructorCall { package: "MyApp::DB" }`.

2. `%hash` slot assignment:

   ```perl
   my %services;
   $services{db} = MyApp::DB->new;
   $services{db}->connect;
   ```

   Assert the same package and evidence as the literal-slot case.

3. Hashref literal slot:

   ```perl
   my $services = {
       db => MyApp::DB->new,
   };

   $services->{db}->connect;
   ```

   Assert package `MyApp::DB`.

4. Dynamic key fails closed:

   ```perl
   my %services = (
       db => MyApp::DB->new,
   );

   $services{$name}->connect;
   ```

   Assert no package and `DynamicBoundary::DynamicHashKey`.

5. Bless object field:

   ```perl
   package MyApp::Service;
   sub new {
       my $class = shift;
       bless { db => MyApp::DB->new }, $class;
   }
   sub run {
       my $self = shift;
       $self->{db}->connect;
   }
   ```

   Assert package `MyApp::DB` and medium confidence.

6. Moo accessor:

   ```perl
   package MyApp::Service;
   use Moo;

   has db => (
       is  => 'ro',
       isa => 'MyApp::DB',
   );

   sub run {
       my $self = shift;
       $self->db->connect;
   }
   ```

   Assert `$self->db` is `Object("MyApp::DB")` and the chained `connect` call resolves against `MyApp::DB`.

### Completion tests

After facts-only tests pass, add LSP completion tests in:

```text
crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs
```

Required assertions:

- `$services{db}->` completion contains `connect` when `connect` is a method on `MyApp::DB`.
- completion detail includes a receiver source such as `receiver: hash slot`.
- completion detail includes confidence such as `high confidence`.
- `$services{$name}->` does not produce exact `MyApp::DB` receiver completions.
- low-confidence unknown fallback, if present, is labeled as fallback and not exact receiver evidence.

## Implementation plan

### PR 1 — Facts model and environment

Files:

```text
crates/perl-semantic-analyzer/src/analysis/type_facts.rs
crates/perl-semantic-analyzer/src/analysis/type_inference.rs
```

Deliver:

- `TypeFact`.
- `ShapeFact`, `HashShape`, `ArrayShape`, `ObjectShape`.
- `TypeEvidence`.
- `DynamicBoundary`.
- `TypeEnvironment::variable_facts`.
- `set_variable_fact`, `get_variable_fact`, and `get_fact_at`.
- Compatibility through `TypeFact::erased_type`.

No completion behavior change.

### PR 2 — Constructors and plain hash slots

Files:

```text
crates/perl-semantic-analyzer/src/analysis/type_inference.rs
crates/perl-semantic-analyzer/tests/receiver_facts.rs
```

Deliver:

- `HashLiteral` to `HashShape`.
- `Class->new` to `PerlType::Object(Class)`.
- `%hash` declarations store shape facts.
- `$hash{key}` resolves static slots.
- `$hash{$dynamic}` returns `DynamicHashKey` and no package.

This is the first PR where `$services{db}` can become `MyApp::DB`.

### PR 3 — Hashref slots

Deliver:

- `$hashref->{key}` shape lookup.
- `$self->{key}` scaffolding through object shapes, even before fields are populated.
- facts tests for `$services->{db}->`.

### PR 4 — Receiver facts API

Files:

```text
crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs
crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs
```

Deliver:

- `ReceiverFact` and `ReceiverExpr`.
- `receiver_fact_for_method_call` / equivalent node API.
- conversion from `ReceiverFact` into completion receiver evidence.
- existing text-pattern classifier remains fallback.

### PR 5 — Completion handoff

Deliver:

- `add_workspace_method_completions` accepts `ReceiverFact` rather than owning receiver semantics.
- exact hash-slot receiver completions work.
- completion detail includes receiver source and confidence.

At this point this should work in completion:

```perl
my %services = (
    db => MyApp::DB->new,
);

$services{db}->  # suggests MyApp::DB methods
```

### PR 6 — Bless/object fields

Deliver:

- `bless { field => Constructor->new }, $class` records object fields.
- `$self->{field}` resolves inside package methods.
- dynamic bless classes fail closed.

### PR 7 — Framework accessor returns

Deliver:

- Moo/Moose `has db => (isa => 'MyApp::DB')` accessor return facts.
- Object::Pad reader/accessor return facts.
- `$self->db->` resolves against the accessor return package.

### PR 8 — Method-return rules

Deliver:

- DBI `connect` / `prepare` facts.
- accessor return chaining.
- simple builder-return inference where safe.
- shared evidence for completion, hover, and future signature help.

## Acceptance criteria

The feature is complete when these cases are covered by facts and LSP completion behavior:

```perl
# 1. hash literal slot
my %services = (db => MyApp::DB->new);
$services{db}->connect;

# 2. hash slot assignment
$services{db} = MyApp::DB->new;
$services{db}->connect;

# 3. hashref literal slot
my $services = { db => MyApp::DB->new };
$services->{db}->connect;

# 4. object field via bless
bless { db => MyApp::DB->new }, $class;
$self->{db}->connect;

# 5. Moo/Moose accessor
has db => (is => 'ro', isa => 'MyApp::DB');
$self->db->connect;

# 6. dynamic key does not lie
$services{$name}->connect;

# 7. dynamic bless does not lie
bless {}, $class;
$self->{db}->connect;
```

Pass/fail distinction:

- Static structure produces precise receiver facts and precise completion.
- Dynamic boundaries produce no exact receiver, or only explicitly labeled low-confidence fallback.
- Every precise completion has evidence.
- Completion details expose enough receiver source and confidence for users and tests to distinguish exact facts from fallback suggestions.

# Receiver Facts Implementation Plan

**Status:** Ready for staged implementation  
**Scope:** Semantic receiver inference for method completion, beginning with static hash/hashref slots  
**Primary crates:** `perl-semantic-analyzer`, `perl-lsp-rs-core`

## Problem

Method completion can already use package and workspace method metadata, but the semantic handoff is too thin for receiver expressions such as `$services{db}->connect` or `$services->{db}->connect`. The parser already preserves the receiver expression shape; the missing piece is a facts layer that joins parsed receiver expressions to typed evidence, package resolution, confidence, and completion ranking.

The target pipeline is intentionally narrow:

```text
parse receiver expression
→ infer receiver fact
→ resolve package/method set
→ rank by confidence
→ prove with fixtures
```

## Existing seams to use

Do not add new AST node variants for the first cut. The current AST already represents the important access forms as binary postfix expressions:

| Perl source form | AST shape to consume |
| --- | --- |
| `$h{k}` | `NodeKind::Binary { op: "{}", left, right }` |
| `$h->{k}` | `NodeKind::Binary { op: "->{}", left, right }` |
| `$a[0]` | `NodeKind::Binary { op: "[]", left, right }` |
| `$a->[0]` | `NodeKind::Binary { op: "->[]", left, right }` |
| `$obj->method` | `NodeKind::MethodCall { object, method, args }` |

The implementation should build semantic facts on top of those shapes rather than changing parser output. Parser readability aliases such as `HashAccess` or `HashRefAccess` can be considered later, after receiver facts are proven.

Relevant current state:

- `TypeEnvironment` stores variable names to erased `PerlType` values and searches parent scopes.
- `TypeInferenceEngine` already recognizes coarse `PerlType` values, including `Hash`, `Reference`, `Object`, `Union`, `Any`, and `Void`.
- `TypeInferenceEngine` already recognizes simple `ClassName->new` constructor calls as `PerlType::Object`.
- `value_shape_inferrer` already carries completion-oriented object-shape heuristics for constructor assignments, DBI handles, `bless`, and self-like receivers.
- Workspace method completion already labels receiver evidence and preserves fallback paths for unknown receivers.

## Non-goals for the first implementation wave

- Do not replace `PerlType`.
- Do not rewrite the parser or add new AST variants.
- Do not remove text-pattern receiver fallback in completion.
- Do not attempt full C3/MRO correctness before receiver facts work.
- Do not make dynamic hash keys appear precise.
- Do not move every DBI or framework heuristic in the first PR.

## Data model specification

Add a facts layer beside the existing erased type model:

```text
crates/perl-semantic-analyzer/src/analysis/type_facts.rs
crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs
```

### `TypeFact`

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

`TypeFact` is the richer semantic currency. Public callers that only understand `PerlType` continue to use `erased_type()`.

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

### Evidence and dynamic boundaries

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

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicBoundary {
    DynamicHashKey,
    DynamicBlessClass,
    DynamicMethodName,
    RuntimeImport,
    UnknownReceiver,
}
```

Every exact receiver completion must carry evidence. Every closed-over dynamic case must carry a boundary, not a guessed package.

## Environment changes

Extend `TypeEnvironment` with a parallel fact map:

```rust
pub struct TypeEnvironment {
    variables: HashMap<String, PerlType>,
    variable_facts: HashMap<String, TypeFact>,
    subroutines: HashMap<String, PerlType>,
    parent: Option<Box<TypeEnvironment>>,
}
```

Add APIs without removing current callers:

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

Existing `set_variable`, `get_variable`, and erased `get_type_at`-style callers remain valid. `set_variable_fact` keeps erased type state synchronized.

## Expression fact inference

Add an expression-level API on `TypeInferenceEngine`:

```rust
impl TypeInferenceEngine {
    pub fn infer_expr_fact(
        &mut self,
        node: &Node,
        env: &mut TypeEnvironment,
    ) -> TypeFact {
        // Dispatch by NodeKind.
    }
}
```

The first implementation pass must handle:

| AST node | Required behavior |
| --- | --- |
| `Variable` | Read `env.get_variable_fact(name)` first, then erase/sigil fallback. |
| `HashLiteral` | Build `HashShape` for static keys and a fallback value fact. |
| `ArrayLiteral` | Build coarse `ArrayShape` and element fallback. |
| `Assignment` with `op == "="` | Infer RHS, update variable or slot facts, return RHS fact. |
| `VariableDeclaration` | Store initializer fact for `%hash`, `$hashref`, and scalar object assignments. |
| `MethodCall` | Recognize constructor and later method-return rules. |
| `FunctionCall` | Recognize `bless` scaffolding later; otherwise use builtins/subroutines. |
| `Binary` with `op == "{}"` | Static plain-hash slot lookup; dynamic key fails closed. |
| `Binary` with `op == "->{}"` | Static hashref/object-field lookup; dynamic key fails closed. |
| `Binary` with `op == "[]"` or `op == "->[]"` | Array slot scaffolding; precise index later. |
| fallback | Low-confidence `Any` with `UnknownReceiver` or contextual boundary when applicable. |

## Static key extraction

Only static keys are precise:

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

If key extraction fails for a hash slot receiver, return a `TypeFact` with `dynamic_boundary: Some(DynamicBoundary::DynamicHashKey)` and no package.

## Hash-shape inference rules

### Hash literal assigned to `%hash`

For:

```perl
my %services = (
    db    => MyApp::DB->new,
    cache => MyApp::Cache->new,
);
```

infer:

- `ty = PerlType::Hash { key: String, value: Union(Object(MyApp::DB), Object(MyApp::Cache)) }`
- `shape = ShapeFact::Hash` with slots `db` and `cache`
- slot facts keep constructor evidence plus `HashSlot { hash: "services", key }`
- confidence is `High` when all precise slot values came from static expressions

### Slot assignment

For:

```perl
my %services;
$services{db} = MyApp::DB->new;
```

`Assignment` should:

1. infer the RHS fact,
2. recognize `Binary { op: "{}" }` on the LHS,
3. extract `(base_name, static_key)`,
4. update `base_name`'s `HashShape`, creating a hash fact if needed,
5. return the RHS fact.

### Plain hash slot lookup

For `$services{db}`, lookup order is:

1. static key extraction,
2. scalarized variable base extraction from `$services`,
3. `env.get_variable_fact("services")`,
4. exact `HashShape.slots["db"]`,
5. `HashShape.fallback_value`,
6. unknown fact.

### Hashref slot lookup

For `$services->{db}`, infer the left side as an expression first. Then:

- if the base has `ShapeFact::Hash`, use hash slot lookup;
- if the base has `ShapeFact::Object`, use object fields;
- otherwise return unknown;
- dynamic keys fail closed with `DynamicHashKey`.

## Constructor and method-return facts

Start with constructor recognition:

```perl
MyApp::DB->new
```

If `method == "new"` and the object is a static package expression, return:

- `ty = PerlType::Object("MyApp::DB")`
- `confidence = High`
- `evidence = ConstructorCall { package: "MyApp::DB" }`

Later method-return rules should move completion-only heuristics into facts:

```rust
pub enum MethodReturnRule {
    ConstructorNew,
    MooseAccessor,
    ObjectPadAccessor,
    DBIConnect,
    DBIPrepare,
}
```

Known first-class returns:

| Expression | Return fact |
| --- | --- |
| `DBI->connect(...)` | `Object("DBI::db")` |
| `$dbh->prepare(...)` | `Object("DBI::st")` when `$dbh` is a DBI db handle |
| `$self->db` | accessor return when class metadata proves `isa => 'MyApp::DB'` |

## Receiver facts API

Completion should consume receiver facts rather than owning type semantics.

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

Add:

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

`receiver_expr_from_node` must preserve enough structure to produce completion detail such as `receiver: hash slot, high confidence`.

## Completion handoff

Keep the current fallback classifier until receiver-fact fixtures prove parity.

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

Inside workspace completion:

```rust
let evidence = receiver_fact
    .map(ReceiverEvidence::from_receiver_fact)
    .unwrap_or_else(|| classify_text_pattern_receiver(context, source));
```

Exact receiver completions should require `receiver_fact.package`. Unknown receiver fallback can remain, but it must stay bounded and be labeled low confidence.

## Object field and accessor extensions

These are follow-on stages after static hash/hashref slots.

### `bless` object fields

For:

```perl
sub new {
    my $class = shift;
    bless { db => MyApp::DB->new }, $class;
}

sub run {
    my $self = shift;
    $self->{db}->connect;
}
```

Infer package object fields when:

- first `bless` argument is a hash literal;
- second argument is a static package string or a `$class` variable tied to the current package;
- `$self`/`$this` is recognized as the invocant in package methods.

Use `Medium` confidence unless the constructor package is static and local.

### Moo/Moose accessors

For:

```perl
has db => (
    is  => 'ro',
    isa => 'MyApp::DB',
);

$self->db->connect;
```

Use class model attributes to infer method return facts:

- find the current package model;
- find the attribute whose accessor/reader matches the method;
- parse simple package `isa` strings;
- return `Object(package)` with `MooseIsa` evidence and `Medium` confidence.

### Object::Pad readers/accessors

Use field metadata such as `reader`, `writer`, `accessor`, `mutator`, initializer, and default evidence to infer method-return facts for `$self->field_reader` chains.

## Staged PR plan

### PR 1 — Facts model and environment

Files:

```text
crates/perl-semantic-analyzer/src/analysis/type_facts.rs
crates/perl-semantic-analyzer/src/analysis/type_inference.rs
```

Deliver:

- `TypeFact`
- `ShapeFact`, `HashShape`, `ArrayShape`, `ObjectShape`
- `TypeEvidence`
- `DynamicBoundary`
- `variable_facts`
- `set_variable_fact`, `get_variable_fact`, `get_fact_at`
- no completion behavior change

### PR 2 — Constructors and plain hash facts

Files:

```text
crates/perl-semantic-analyzer/src/analysis/type_inference.rs
crates/perl-semantic-analyzer/tests/receiver_facts.rs
```

Deliver:

- `infer_expr_fact`
- `HashLiteral` to `HashShape`
- `Class->new` to `Object(Class)` fact
- `%hash` declaration shape storage
- `$hash{key}` static slot lookup
- `$hash{$dynamic}` fail-closed boundary

### PR 3 — Hashref slots

Deliver:

- `$hashref->{key}` lookup for scalar hashref literals
- initial `$self->{key}` scaffolding without requiring object field population
- tests for `$services->{db}->connect`

### PR 4 — Receiver facts API

Files:

```text
crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs
crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs
```

Deliver:

- `ReceiverFact`
- `ReceiverExpr`
- `receiver_fact_for_method_call`
- conversion to existing `ReceiverEvidence`
- text-pattern fallback retained

### PR 5 — Completion handoff

Deliver:

- method completion accepts `ReceiverFact`
- exact hash-slot receiver completions work
- completion detail includes receiver kind, confidence, and evidence
- unknown/dynamic receiver fallback remains low confidence

### PR 6 — `bless` object fields

Deliver:

- `bless { field => Constructor->new }, $class` field facts
- `$self->{field}` resolution inside package methods
- dynamic bless class fails closed

### PR 7 — Framework accessor returns

Deliver:

- Moo/Moose `has db => (isa => 'MyApp::DB')`
- Object::Pad reader/accessor returns
- `$self->db->` receiver facts

### PR 8 — Shared method-return rules

Deliver:

- DBI connect/prepare as facts
- accessor return chaining
- simple builder-return inference
- completion, hover, and signature help can share return evidence

## Facts-only fixture plan

Add:

```text
crates/perl-semantic-analyzer/tests/receiver_facts.rs
```

Start without LSP protocol assertions.

| Test | Fixture | Expected fact |
| --- | --- | --- |
| hash literal slot | `my %services = (db => MyApp::DB->new); $services{db}->connect;` | package `MyApp::DB`, `High`, `HashSlot`, `ConstructorCall` |
| hash slot assignment | `my %services; $services{db} = MyApp::DB->new; $services{db}->connect;` | same as hash literal slot |
| hashref literal slot | `my $services = { db => MyApp::DB->new }; $services->{db}->connect;` | package `MyApp::DB` |
| dynamic key | `$services{$name}->connect;` | no package, `DynamicHashKey` |
| bless object field | `bless { db => MyApp::DB->new }, $class; $self->{db}->connect;` | package `MyApp::DB`, `Medium` |
| Moose accessor | `has db => (is => 'ro', isa => 'MyApp::DB'); $self->db->connect;` | `$self->db` is `Object(MyApp::DB)` |

## LSP completion fixture plan

After facts-only fixtures pass, add completion-provider tests that assert labels and detail strings:

```text
crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs
```

Required assertions:

- `$services{db}->` completes methods from `MyApp::DB`.
- detail includes `receiver: hash slot` and confidence.
- `$services->{db}->` completes methods from `MyApp::DB`.
- `$services{$name}->` does not produce an exact `MyApp::DB` receiver completion.
- unknown fallback, if present, is labeled `receiver: unknown, low confidence`.

## Acceptance criteria

The receiver-facts feature is complete when these source patterns are proven by facts and completion fixtures:

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

- Static structure produces precise completion.
- Dynamic boundaries produce no exact receiver, or bounded low-confidence fallback only.
- Every precise completion includes evidence.

## Review checklist

Before each implementation PR is ready for review:

- The PR touches one phase only.
- Existing erased `PerlType` APIs still compile.
- No parser AST compatibility break is introduced.
- Dynamic key and dynamic bless fixtures fail closed.
- Exact receiver completions have visible confidence/evidence detail.
- Text-pattern completion fallback remains until replacement fixtures prove parity.

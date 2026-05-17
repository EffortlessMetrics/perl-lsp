# Receiver Facts Pipeline Specification

This document lays down the implementation plan for precise method completions on receiver expressions such as `$services{db}->`, `$services->{db}->`, `$self->{field}->`, and framework accessors. The scope is intentionally narrow: keep the existing parser shapes, add expression facts over the AST, and feed those facts into workspace method completion.

## Problem Statement

Method completion currently has pieces of the answer in different layers, but the layers do not share a typed receiver contract:

```text
parse receiver expression
→ infer receiver fact
→ resolve package/method set
→ rank by confidence
→ prove with fixtures
```

The missing join is expression-level semantic evidence. In particular, the parser already represents postfix hash and array access as `NodeKind::Binary` operators (`"{}"`, `"->{}"`, `"[]"`, and `"->[]"`). The first implementation must use those existing AST forms rather than introducing new AST node variants.

## Non-Goals

- Do not rewrite the parser or add dedicated `HashAccess` / `HashRefAccess` / `ArrayAccess` AST variants in the first wave.
- Do not replace `PerlType`; keep it as the erased public type representation while richer facts grow beside it.
- Do not cut completion over without facts-only regression tests.
- Do not remove existing text-pattern receiver fallback logic until fixture evidence proves the fact pipeline.
- Do not block the first hash-slot cut on full C3 MRO, framework accessor inference, or DBI return rules.

## Current Seams To Reuse

| Existing seam | Role in this plan |
| --- | --- |
| `crates/perl-ast/src/ast.rs` | Source of the existing `NodeKind` expression shapes. |
| `crates/perl-semantic-analyzer/src/analysis/type_inference.rs` | Home for `PerlType`, `TypeEnvironment`, and the first expression-fact inference implementation. |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/workspace.rs` | Current workspace method completion path and receiver fallback heuristics. |
| `crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs` | Existing receiver-evidence tests and future completion-level receipts. |

## Data Model: Type Facts

Add a facts layer in `crates/perl-semantic-analyzer/src/analysis/type_facts.rs`.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TypeFact {
    pub ty: PerlType,
    pub confidence: Confidence,
    pub evidence: Vec<TypeEvidence>,
    pub dynamic_boundary: Option<DynamicBoundary>,
    pub shape: Option<ShapeFact>,
}
```

The initial fact vocabulary is:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFact {
    Hash(HashShape),
    Array(ArrayShape),
    Object(ObjectShape),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HashShape {
    pub slots: std::collections::BTreeMap<String, TypeFact>,
    pub fallback_value: Option<Box<TypeFact>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayShape {
    pub indexed: std::collections::BTreeMap<usize, TypeFact>,
    pub element: Option<Box<TypeFact>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectShape {
    pub package: String,
    pub fields: std::collections::BTreeMap<String, TypeFact>,
}
```

Evidence must be explicit because precise completions should explain why a receiver was trusted:

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

Compatibility rule: `PerlType` remains the erased type representation.

```rust
impl TypeFact {
    pub fn erased_type(&self) -> PerlType {
        self.ty.clone()
    }
}
```

Recommended constructors:

- `TypeFact::unknown()` returns `PerlType::Any` with low confidence and `UnknownReceiver` when appropriate.
- `TypeFact::any_low_confidence(reason)` returns `PerlType::Any` with heuristic evidence.
- `TypeFact::dynamic(boundary)` returns `PerlType::Any`, low confidence, and the given boundary.
- `TypeFact::unknown_hash()` returns a hash with string keys, `Any` values, no slots, and low confidence.

## Environment Contract

Extend `TypeEnvironment` with a parallel fact map while keeping the existing variable type map for compatibility:

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

`set_variable(name, ty)` should continue to work. It may optionally populate a low-confidence `TypeFact` wrapper, but it must not discard a richer existing fact for the same variable unless the assignment really overwrites it.

## Expression Fact Inference

Add an expression-level fact entry point to `TypeInferenceEngine`:

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

The first implementation must cover these AST forms:

```rust
match &node.kind {
    NodeKind::Variable { sigil, name } => { /* env fact lookup */ }
    NodeKind::HashLiteral { pairs } => { /* HashShape */ }
    NodeKind::ArrayLiteral { elements } => { /* ArrayShape */ }
    NodeKind::Assignment { lhs, rhs, op } if op == "=" => { /* update env */ }
    NodeKind::VariableDeclaration { variable, initializer, .. } => { /* store fact */ }
    NodeKind::MethodCall { object, method, args } => { /* constructor and returns */ }
    NodeKind::FunctionCall { name, args } => { /* bless later */ }
    NodeKind::Binary { op, left, right } if op == "{}" => { /* hash slot */ }
    NodeKind::Binary { op, left, right } if op == "->{}" => { /* hashref slot */ }
    NodeKind::Binary { op, left, right } if op == "[]" || op == "->[]" => { /* array slot */ }
    _ => TypeFact::any_low_confidence("unsupported expression fact"),
}
```

### Static Key Helper

Only static keys produce precise slot facts in the first cut:

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

Dynamic keys must fail closed:

```rust
$services{$name}->connect;
```

Expected fact: no package, low confidence, and `DynamicBoundary::DynamicHashKey`.

## Hash Shape Inference

### Hash Literal Assigned To A Hash

For:

```perl
my %services = (
    db    => MyApp::DB->new,
    cache => MyApp::Cache->new,
);
```

Infer:

- `PerlType::Hash { key: Scalar(String), value: Union(Object(MyApp::DB), Object(MyApp::Cache)) }`
- `ShapeFact::Hash` with exact `db` and `cache` slots.
- `Confidence::High` when all keys are static and values are high-confidence constructors.
- `TypeEvidence::Literal`, `TypeEvidence::VariableInitializer { name: "services" }`, per-slot `HashSlot`, and per-value `ConstructorCall` evidence.

### Slot Assignment

For:

```perl
my %services;
$services{db} = MyApp::DB->new;
$services{db}->connect;
```

`infer_expr_fact` on the assignment must update `%services` through the scalarized slot syntax. The helper should recognize `NodeKind::Binary { op: "{}" }` where the left node is `NodeKind::Variable { sigil: "$", name }` and the right node has a static key. The environment update must insert or refine the hash shape slot and refresh the erased hash value type.

### Plain Hash Slot Lookup

For `NodeKind::Binary { op: "{}" }`:

1. Resolve a static key.
2. Require a scalarized hash variable receiver such as `$services{db}`.
3. Look up the base fact by variable name `services`.
4. Return the exact slot fact when present.
5. Otherwise return the hash fallback value when present.
6. Otherwise return unknown.

The returned slot fact should carry slot evidence that names the base hash and key, not only the literal hash evidence.

## Hashref Shape Inference

For:

```perl
my $services = {
    db => MyApp::DB->new,
};

$services->{db}->connect;
```

The first hashref cut should treat a hash literal assigned to a scalar as a `PerlType::Reference(Box::new(PerlType::Hash { ... }))` or as a fact whose erased type is compatible with current scalar behavior and whose shape is `ShapeFact::Hash`. `NodeKind::Binary { op: "->{}" }` should infer the left expression first and then resolve the key against `ShapeFact::Hash`.

For later object support, the same `"->{}"` path should also check `ShapeFact::Object(obj)` and return `obj.fields[key]`.

## Constructor Recognition

`NodeKind::MethodCall { object, method, args }` should produce high-confidence constructor facts for static package calls:

```perl
MyApp::DB->new
```

Rule:

```rust
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
```

`static_package_expr` should accept package-looking identifiers such as `MyApp::DB` and reject variables or dynamically constructed class names.

## Receiver Facts API

Add `crates/perl-semantic-analyzer/src/analysis/receiver_facts.rs` after expression facts are useful.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverFact {
    pub receiver: ReceiverExpr,
    pub fact: TypeFact,
    pub package: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReceiverExpr {
    StaticPackage(String),
    Variable(String),
    HashSlot { base: String, key: String },
    HashRefSlot { base: Box<ReceiverExpr>, key: String },
    MethodCall { receiver: Box<ReceiverExpr>, method: String },
    Unknown,
}
```

Entry point:

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

## Completion Handoff

Workspace method completion should consume receiver facts when available and keep text-pattern classification as fallback.

Target call shape:

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

Rules:

- If `receiver_fact.package` is `Some(package)`, method candidates should be resolved against that package first.
- Exact receiver completions should include confidence and evidence in detail or documentation.
- If a dynamic boundary is present, do not emit exact receiver completions; only existing low-confidence fallback candidates may remain.
- Existing `classify_receiver` / text-pattern tests stay in place until replacement receipts exist.

## Object Field Follow-Up

After hash and hashref slots work, infer fields from clean `bless` literals:

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

Implementation notes:

1. Recognize `FunctionCall { name: "bless", args }` when the first argument is a hash literal.
2. Resolve the package from a static string or a clean `$class = shift` constructor pattern tied to the current package.
3. Convert the hash shape to `ObjectShape { package, fields }`.
4. Store package object facts separately from local variables.
5. Resolve `$self->{field}` inside methods of that package with medium confidence.
6. Dynamic bless classes produce `DynamicBoundary::DynamicBlessClass` and must not claim exact fields.

## Framework Accessor Follow-Up

Moo/Moose/Object::Pad support should be implemented as method-return facts after receiver facts are in place.

Initial rules:

| Pattern | Return fact |
| --- | --- |
| `has db => (is => 'ro', isa => 'MyApp::DB')` and `$self->db` | `Object("MyApp::DB")`, medium confidence, `MooseIsa` evidence. |
| `field $db :reader` / `:accessor` and `$self->db` | Field type or initializer-derived type, medium confidence, `ObjectPadField` evidence. |
| `DBI->connect(...)` | `Object("DBI::db")`, heuristic confidence until backed by workspace facts. |
| `$dbh->prepare(...)` | `Object("DBI::st")`, heuristic confidence until backed by receiver facts. |

These rules should live in semantic analysis so completion, hover, and future signature help share the same evidence.

## Method Resolution Follow-Up

The first receiver-facts implementation may continue using the current workspace method collection behavior. Once precise receiver facts are flowing, add a method candidate resolver:

```rust
resolve_method_candidates(package, method_prefix, mro_mode)
```

The resolver should eventually honor `ClassModel.mro` for DFS versus C3 ordering, but MRO correctness is not a blocker for `$hash{key}->` precision.

## Tests And Receipts

### Facts-Only Tests

Add `crates/perl-semantic-analyzer/tests/receiver_facts.rs` before changing completion behavior.

Required cases:

1. `%hash` literal slot:

   ```perl
   package MyApp::DB;
   sub connect {}

   package main;
   my %services = (db => MyApp::DB->new);
   $services{db}->connect;
   ```

   Assert package `MyApp::DB`, high confidence, `HashSlot { hash: "services", key: "db" }`, and `ConstructorCall { package: "MyApp::DB" }`.

2. `%hash` slot assignment:

   ```perl
   my %services;
   $services{db} = MyApp::DB->new;
   $services{db}->connect;
   ```

   Assert the same package and evidence class as the literal case.

3. Hashref literal slot:

   ```perl
   my $services = { db => MyApp::DB->new };
   $services->{db}->connect;
   ```

   Assert package `MyApp::DB`.

4. Dynamic key fails closed:

   ```perl
   my %services = (db => MyApp::DB->new);
   $services{$name}->connect;
   ```

   Assert no exact package and `DynamicBoundary::DynamicHashKey`.

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

   Assert package `MyApp::DB` with medium confidence after the object-field PR.

6. Moose accessor:

   ```perl
   package MyApp::Service;
   use Moo;
   has db => (is => 'ro', isa => 'MyApp::DB');
   sub run { my $self = shift; $self->db->connect; }
   ```

   Assert `$self->db` is `Object("MyApp::DB")` after the framework accessor PR.

### Completion Tests

After facts-only receipts pass, add completion tests in `crates/perl-lsp-rs-core/src/providers/completion/completion/tests.rs`:

- `assert_completion_contains(code, "$services{db}->", "connect")`.
- Completion detail or documentation contains receiver kind, confidence, and evidence.
- `assert_completion_not_exact_receiver(code, "$services{$name}->", "connect")`.
- Existing unknown fallback may still appear, but it must be clearly low confidence.

## PR Sequence

| PR | Scope | Main deliverables |
| --- | --- | --- |
| 1 | Facts model and environment | `TypeFact`, shapes, evidence, dynamic boundaries, `variable_facts`, `get_fact_at`; no completion change. |
| 2 | Constructors and plain hashes | `HashLiteral` to `HashShape`, `Class->new` to object fact, `%hash` declaration and `$hash{key}` assignment/lookup, dynamic-key fail-closed tests. |
| 3 | Hashref slots | `$hashref->{key}` shape lookup and `$self->{key}` scaffolding with tests. |
| 4 | Receiver facts API | `receiver_facts.rs`, `ReceiverFact`, `ReceiverExpr`, and method-call receiver extraction. |
| 5 | Completion handoff | Workspace method completion consumes `ReceiverFact`, preserves fallback, and labels confidence/evidence. |
| 6 | Bless/object fields | `bless { field => Constructor->new }, $class` object field facts and `$self->{field}` resolution. |
| 7 | Framework accessors | Moo/Moose `has ... isa` and Object::Pad reader/accessor return facts. |
| 8 | Method return rules | DBI connect/prepare, accessor chaining, and simple builder-return inference as shared facts. |

## Acceptance Criteria

This lane is complete when static receiver structure produces exact, evidence-backed completions and dynamic structure fails closed:

```perl
my %services = (db => MyApp::DB->new);
$services{db}->connect;

$services{db} = MyApp::DB->new;
$services{db}->connect;

my $services = { db => MyApp::DB->new };
$services->{db}->connect;

bless { db => MyApp::DB->new }, $class;
$self->{db}->connect;

has db => (is => 'ro', isa => 'MyApp::DB');
$self->db->connect;

$services{$name}->connect;
bless {}, $class;
$self->{db}->connect;
```

Pass/fail distinction:

- Static structure produces precise package resolution and method completions.
- Dynamic boundaries do not claim exact receiver packages.
- Every precise completion carries confidence and evidence.
- The old text fallback remains available only as a low-confidence fallback until intentionally retired.

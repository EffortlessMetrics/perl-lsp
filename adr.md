# ADR-0087: Rose::DB::Object IDE Support via Framework Enum

## Status

**Accepted**

## Context

The issue [request] asks for IDE support (navigation and completion) for Rose::DB::Object, a popular Perl ORM. Specifically:
- Recognition of classes inheriting from Rose::DB::Object
- Completion for auto-generated column accessors (e.g., `id()`, `name()`, `email()`)
- Go-to-definition for column definitions in `meta->setup(...)`

Rose::DB::Object uses `use base qw(Rose::DB::Object)` for inheritance and `__PACKAGE__->meta->setup(columns => [...])` for schema definition. This differs from Moose/Moo/Mouse which use `has` declarations.

### Existing Framework Detection

The codebase has a `Framework` enum in `class_model.rs` that tracks which OO framework a package uses:
- `Moose`, `Moo`, `Mouse` - via `use Moose;`, `use Moo;`, etc.
- `ClassAccessor` - via `use Class::Accessor` or `use base qw(Class::Accessor)`
- `ObjectPad` - via `use Object::Pad`
- `Native`, `NativeClass` - native Perl OO
- `PlainOO` - plain inheritance via `use base`/`use parent` with no known framework
- `None` - no OO detected

### Two-System Architecture

The codebase has **two independent framework detection systems**:

1. **`class_model.rs`**: `Framework` enum, `ClassModel`, `ClassModelBuilder` - used for semantic analysis and navigation
2. **`symbol.rs`**: `FrameworkFlags` with `moo: bool`, `class_accessor: bool`, `kind: Option<FrameworkKind>` - used for symbol extraction and completion

Rose::DB::Object support requires updates to both systems.

### Design Tension

The vision alignment review raised a valid architectural concern: adding `RoseDBObject` to the `Framework` enum conflates two distinct concepts:
- **Method-declaration frameworks** (Moose, Moo, Mouse): track *how* methods are declared at compile time
- **Runtime schema conformance** (Rose::DB::Object): tracks *what* runtime schema a class conforms to

The `Framework` enum's documented purpose is "Which OO framework a package uses" for "Moose/Moo/Mouse/Class::Accessor intelligence." Rose::DB::Object doesn't declare methods via `has` or `field` - it introspects a runtime data structure.

## Decision

**Add `RoseDBObject` to the `Framework` enum, with explicit documentation of its semantic distinction.**

### Rationale

1. **Follows existing patterns**: The `base | parent` branch in `detect_framework()` already captures parent class names. Adding Rose::DB::Object detection to this branch is a natural extension.

2. **Enables both use cases**: A pure DBI-style approach (completion only, no Framework enum) would not support go-to-definition/navigation which requires framework detection.

3. **Two-system architecture requires it**: Even if we use DBI-style completion, `class_model.rs` still needs to detect Rose::DB::Object for navigation. Splitting the detection would create inconsistency.

4. **Mitigatable technical debt**: The semantic conflation concern is valid but can be addressed through:
   - Clear documentation in the `Framework` enum that RoseDBObject represents "runtime schema conformance"
   - Not adding a `rose_columns` field to `ClassModel` (keep it in attributes)
   - Isolating Rose::DB::Object extraction logic within `class_model.rs`

5. **Future precedent**: If DBIx::Class support is needed later, the same question arises. We establish that ORMs belong in the enum, with documented semantics.

### Detection Strategy

In `detect_framework()`, when processing `use base qw(... Rose::DB::Object ...)`:
1. Capture `Rose::DB::Object` in `current_parents`
2. If no stronger framework detected, set `Framework::RoseDBObject` instead of `Framework::PlainOO`
3. This works for single-file analysis when `Rose::DB::Object` is in the `use base` args

### What Goes in Framework Enum

```rust
/// Rose::DB::Object ORM schema conformance.
/// Uses `meta->setup(columns => [...])` for runtime accessor synthesis,
/// not compile-time `has`/`field` declarations.
RoseDBObject,
```

### What Does NOT Go In

- `rose_columns` field in `ClassModel` - keep column info in `attributes` with `declaration = "meta->setup"`
- Relationship navigation (out of scope for initial implementation)
- Manager query patterns (out of scope)

## Consequences

### Positive
- Detection and completion work consistently across both systems
- Navigation support enabled via ClassModel
- Follows established patterns, lower implementation complexity
- Incremental - simple column extraction first, relationships later

### Negative / Tradeoffs
- Semantic conflation: Framework enum now tracks both "method-declaration" and "runtime schema conformance" concepts
- Future ORM additions (DBIx::Class) will face same design question
- `methods.rs` doesn't have access to `ClassModel` - completion requires workspace index lookup or extending CompletionContext

### Risks

1. **Cross-file detection**: In single-file analysis, `Rose::DB::Object` base class isn't defined. Mitigation: Use workspace index for cross-file parent chain resolution.

2. **Chained method call AST**: `__PACKAGE__->meta->setup(...)` is a nested `MethodCall { object: MethodCall {...}, method: "setup", args: [...] }`. Mitigation: Start with `__PACKAGE__->meta->setup(...)` only.

3. **Synthetic method conflicts**: User might define their own `id()` method. Mitigation: Mark synthesized methods distinctly, deprioritize in completion ranking.

## Alternatives Considered

### Alternative 1: DBI-Style Completion-Localized (Rejected)

Keep Rose::DB::Object classes as `Framework::PlainOO`, add schema extraction to separate module, wire completion via `infer_receiver_type()`.

**Rejected because**:
- No existing DBI-style ORM precedent - would be first such pattern
- `infer_receiver_type()` only does variable naming heuristics, not parent chain
- Doesn't support navigation/go-to-definition
- SymbolExtractor's `update_framework_context()` would still need Rose::DB::Object detection

### Alternative 2: Hybrid - Framework Enum + Separate Extraction (Rejected)

Add RoseDBObject to Framework enum but extract schema in `rose_db.rs` module, wire completion via workspace index.

**Rejected because**:
- Adds complexity (separate module) without clear benefit
- Both `class_model.rs` and `symbol.rs` still need Rose::DB::Object detection anyway
- Over-engineering for initial scope

## Implementation Notes

### Changes Required

1. **`class_model.rs`**:
   - Add `RoseDBObject` to `Framework` enum with documentation
   - Update `detect_framework()` in the `base | parent` branch to check for `Rose::DB::Object` in parents and set `Framework::RoseDBObject`
   - Create `RoseDBObjectColumn` struct for extracted column metadata
   - Add `try_extract_meta_setup()` function for chained method call AST pattern
   - Store extracted columns in `ClassModel::attributes` with `declaration = "meta->setup"`

2. **`symbol.rs`**:
   - Add `rose_db_object: bool` to `FrameworkFlags`
   - Update `update_framework_context()` to detect Rose::DB::Object inheritance
   - Register synthesized accessor symbols when Rose::DB::Object schema is extracted

3. **`methods.rs`**:
   - Extend completion inference to recognize Rose::DB::Object subclasses via workspace index or parent chain lookup
   - Inject column accessor completions

4. **`declaration.rs`** (navigation):
   - Enable go-to-definition for synthesized accessors → `meta->setup(...)` call
   - Navigate to column definition within `meta->setup(...)`

### Initial Scope

In scope:
- Detection via `use base qw(... Rose::DB::Object ...)`
- `columns => [qw(id name email)]` extraction (qw() form only)
- Column accessor completion (`id()`, `name()`, etc.)
- Go-to-definition on column accessor → `meta->setup(...)` call

Out of scope:
- Relationships (one_to_many, many_to_many)
- Manager query patterns
- `accessor => 'custom_name'` overrides
- Variable references in columns (`columns => $array`)
- Cross-file schema resolution (use workspace index where needed)
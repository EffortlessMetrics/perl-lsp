# ADR/Spec Findings — work-cb980638

## What This ADR Decides

Whether to add Rose::DB::Object support via the existing Framework enum (like Moose/Moo/Mouse) or via a DBI-style completion-localized approach. This decision affects the semantic meaning of the Framework enum and the complexity of implementation.

## Key Decision

**Decision: Add RoseDBObject to the Framework enum, but document its semantic distinction as "runtime schema conformance" vs "method-declaration framework".**

Rationale:
1. The existing codebase has no precedent for DBI-style ORM detection - all ORMs/schema systems would need a new pattern
2. The initial plan explicitly chose the Framework enum approach after research
3. The vision alignment concern is valid but can be mitigated with documentation
4. The two-system architecture (class_model.rs + symbol.rs) requires Framework detection anyway for navigation
5. Rose::DB::Object detection via `use base qw(Rose::DB::Object)` naturally fits the existing `base | parent` branch

## Alternatives Considered

### Alternative 1: DBI-Style Completion-Localized Approach
Keep Rose::DB::Object classes as `Framework::PlainOO`, add Rose::DB::Object schema extraction to a separate module, wire completion via `infer_receiver_type()` style inference.

**Rejected because**:
- No existing DBI-style ORM precedent in codebase
- Would require significant new infrastructure (infer_receiver_type doesn't know about parent chains)
- SymbolExtractor's `update_framework_context()` doesn't have parent chain access
- Doesn't support go-to-definition/navigation use cases

### Alternative 2: Hybrid - Framework Enum + Isolated Extraction
Add RoseDBObject to Framework enum but extract schema in a separate `rose_db.rs` module, wire completion via class hierarchy lookup in workspace index.

**Rejected because**:
- Adds complexity without clear benefit over the simpler approach
- The Framework enum approach is already simpler and follows established patterns

## Consequences

### Benefits
- Follows existing Framework enum pattern used by Moose/Moo/Mouse
- Detection via `use base qw(Rose::DB::Object)` fits naturally into existing `detect_framework()` logic
- Synthesized accessors with `declaration = "meta->setup"` follows existing symbol.rs pattern
- Enables both completion AND navigation use cases
- Incremental scope - can start simple, extend later

### Tradeoffs / Technical Debt
- Semantic conflation: Framework enum historically meant "method-declaration framework", RoseDBObject is "runtime schema conformance"
- Future ORM additions (DBIx::Class, etc.) may face same design question
- `methods.rs` doesn't have direct access to `ClassModel` - completion requires workspace index or parent chain lookup

### Risks
- Cross-file detection: In single-file analysis, `Rose::DB::Object` base class isn't defined in the file
- Mitigation: Use workspace index for cross-file parent chain resolution
- Chained method call AST: `__PACKAGE__->meta->setup(...)` is a nested MethodCall, not simple pattern
- Mitigation: Start with `__PACKAGE__->meta->setup(...)` only, extend later

## Acceptance Criteria

From specs.md:
1. Classes using `use base qw(Rose::DB::Object)` are detected as `Framework::RoseDBObject`
2. Column accessors (e.g., `id()`, `name()`, `email()`) appear in method completion after `->`
3. Go-to-definition on a column accessor navigates to the `meta->setup(...)` call
4. `meta->setup(...)` extraction handles `columns => [qw(id name email)]` pattern
5. Synthesized methods are marked with `declaration = "meta->setup"` in symbol table
6. `cargo test -p perl-semantic-analyzer` and `cargo test -p perl-lsp-completion` pass
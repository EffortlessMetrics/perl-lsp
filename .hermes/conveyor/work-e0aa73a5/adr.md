# ADR: Schema Migration File Discovery for perl-lsp

## Status
**Proposed**

---

## Context

The issue [feat: Schema migration support (DBIx::Class::DeploymentHandler, etc.) #3564](https://github.com/EffortlessMetrics/perl-lsp/issues/3564) requests IDE support for Perl database schema migration tools. The issue has three phases:
1. **Phase 1**: File pattern recognition
2. **Phase 2**: Navigation between migration files
3. **Phase 3**: DSL completion for DeploymentHandler

The initial plan proposed implementing all of Phase 1 including SQL highlighting and document links. However, review agents identified critical architectural flaws:

### Problem 1: SQL Highlighting Cannot "Leverage" Existing Infrastructure
The semantic tokens pipeline requires a Perl AST (`doc.ast`). When a `.sql` file is opened:
- `text_sync.rs` attempts to parse `.sql` content as Perl — producing garbage
- `tokenize_sql_body()` only fires via `heredoc_injection_language()` which recognizes `<<SQL`, `<<MYSQL`, etc.
- Standalone `.sql` files never trigger this path

Claiming SQL highlighting "leverages existing infrastructure" is architecturally incorrect.

### Problem 2: `.sql` in `PERL_SOURCE_EXTENSIONS` Is a Semantic Error
`PERL_SOURCE_EXTENSIONS` is documented as "canonical Perl source file extensions." Adding `.sql` to this list conflates two distinct file types and confuses future maintainers.

### Problem 3: Document Links for Migration Files Is New Feature Work
`compute_links()` only handles `use`/`require` statements. Detecting inline migration file paths like `"share/deploy/1.001/001-auto.sql"` requires a new detection pattern, not an extension.

### Problem 4: Discovery Pipeline Over-Discovery Risk
The plan proposed gating `.sql` discovery on migration directory context, but `is_perl_discovery_path()` only receives a `&Path` with no additional directory context.

---

## Decision

**Implement Phase 1 as file discovery only**, with the following specific choices:

### 1. Create a Separate `is_migration_discovery_path()` Function

Do NOT add `.sql` to `PERL_SOURCE_EXTENSIONS`. Instead, create an orthogonal discovery path in `crates/perl-workspace-index/src/discovery/mod.rs`:

```rust
pub fn is_migration_discovery_path(path: &Path) -> bool {
    // Check path components for migration directory patterns
    // share/deploy/, share/upgrade/, share/revert/, sqitch.plan, etc.
}
```

Discovery then calls:
```rust
if is_perl_discovery_path(path) || is_migration_discovery_path(path) {
    files.push(path);
}
```

### 2. Limit Phase 1 to File Discovery

**In scope for Phase 1:**
- Discovery of `.sql` files in migration directories (via `is_migration_discovery_path()`)
- Discovery of `sqitch.plan` files
- Discovery of deployment handler paths: `share/deploy/`, `share/upgrade/`, `share/revert/`

**Out of scope for Phase 1 (deferred to Phase 2):**
- SQL syntax highlighting for standalone `.sql` files (requires new tokenization path)
- Document links for migration file references (requires new detection pattern)
- Navigation between migration files

### 3. Accept Fragile Path Component Checking

The `is_migration_discovery_path()` function checks path components for known migration directory names. This is fragile but workable for Phase 1:
- `share/deploy/`, `share/upgrade/`, `share/revert/` (DeploymentHandler)
- `deploy/`, `verify/`, `revert/` (sqitch-style)
- `sqitch.plan` as a recognized filename

---

## Alternatives Considered

### Alternative 1: Full SQL Highlighting Pipeline (Rejected)
Build a separate SQL document handler with its own tokenizer. **Rejected** because:
- Significant engineering effort
- Adds maintenance burden with two highlighting pipelines
- SQL highlighting is available natively in all major editors
- This complexity is not justified for Phase 1

### Alternative 2: Add `.sql` to `PERL_SOURCE_EXTENSIONS` (Rejected)
**Rejected** because:
- Semantic error: SQL is not Perl source
- Will confuse future maintainers
- Makes the Perl source concept dirty
- Difficult to undo once merged

### Alternative 3: Defer Entirely (Rejected)
**Rejected** because:
- File discovery alone provides immediate value (IDE can show migration files in file tree)
- Phased approach allows proper architectural design at each step
- Users with migration workflows need file discovery now

### Alternative 4: Global `.sql` Discovery Without Context Filtering (Rejected)
**Rejected** because:
- Would index all `.sql` files including fixtures, documentation, test data
- Noisy for users who don't use migration tools
- Over-breadth reduces LSP performance

---

## Consequences

### Positive
1. **Architecturally sound**: Perl source concept stays clean; migration discovery is orthogonal
2. **Low risk**: No modification to Perl parser, semantic analyzer, or tokenization pipeline
3. **Stable**: Does not expand LSP surface area in ways that create technical debt
4. **Phased**: Future phases can address SQL highlighting and navigation with proper design

### Negative
1. **Limited immediate value**: Users only get file discovery, not SQL highlighting or navigation
2. **Fragile path matching**: Path component checking may miss edge cases (e.g., `db/core/sqitch.plan`)
3. **No editor-native SQL highlighting**: `.sql` files opened in the editor won't get Perl LSP SQL highlighting

### Tradeoffs
| Aspect | Decision | Rationale |
|--------|----------|-----------|
| SQL highlighting | Deferred to Phase 2 | Architectural complexity too high for Phase 1 |
| `.sql` in PERL_SOURCE_EXTENSIONS | Rejected | Semantic error; use separate discovery path |
| Document links | Deferred to Phase 2 | New feature work, not extension |
| Path filtering | Accept fragility | Path component checking is workable for Phase 1 |

---

## Dependencies

1. `is_migration_discovery_path()` must verify `share/` is not in the skip list (`path_contains_skipped_component()`)
2. Phase 2 will need to properly design SQL highlighting architecture
3. Phase 2 will need to design migration-to-migration navigation

---

## Future Work (Phase 2)

1. **SQL Highlighting**: Design a separate SQL tokenization path that doesn't depend on Perl AST
2. **Document Links**: Add migration file reference detection to `compute_links()`
3. **Navigation**: Add commands to jump between related migration files

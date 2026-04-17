# Initial Plan — work-e0aa73a5

## Issue
[feat: Schema migration support (DBIx::Class::DeploymentHandler, etc.) #3564](https://github.com/EffortlessMetrics/perl-lsp/issues/3564)

---

## Approach

Implement Phase 1 of the issue: **File pattern recognition** for Perl database migration tools (DBIx::Class::DeploymentHandler, sqitch).

This is the safest starting point because:
1. It does not require modifying the Perl parser or semantic analyzer
2. It leverages existing infrastructure (document links, semantic tokens)
3. It provides immediate value (file discovery + syntax highlighting)
4. It establishes patterns that Phase 2 (navigation) and Phase 3 (completion) can build upon

### Specific Changes

#### 1. Extend File Discovery for Migration Files

**File**: `crates/perl-source-file/src/lib.rs`
- Add `.sql` to `PERL_SOURCE_EXTENSIONS`
- Update `is_perl_source_extension()` and `is_perl_source_path()` to handle SQL files

**File**: `crates/perl-workspace-index/src/discovery/mod.rs`
- Modify `is_perl_discovery_path()` to also recognize migration-specific paths:
  - Paths containing `share/deploy/`, `share/upgrade/`, `share/revert/` (DeploymentHandler)
  - Paths containing `deploy/`, `verify/`, `revert/` (sqitch-style)
  - Files named `sqitch.plan`

#### 2. Add SQL Syntax Highlighting for .sql Files

**File**: `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs`
- Detect when a document has a `.sql` extension or is in a migration directory
- Apply `SQL_KW_RE` regex to highlight SQL keywords (SELECT, FROM, WHERE, etc.)
- Leverage existing `sql_heredoc_keyword` infrastructure

#### 3. Add Document Links for Migration Files

**File**: `crates/perl-lsp-document-links/src/lib.rs`
- Add detection for migration file references within Perl files:
  - Strings containing version paths like `share/deploy/1.001/`
  - Strings referencing `sqitch.plan`
- Create links between deploy/upgrade/revert files within the same version chain

#### 4. Update Feature Catalog

**File**: `features.toml`
- Add feature entries for new capabilities:
  - `lsp.migration_file_discovery`
  - `lsp.sql_highlighting` (or extend existing `lsp.semantic_tokens`)
  - `lsp.migration_document_links`

---

## Risks

### Risk 1: SQL File Over-Discovery
**Problem**: Adding `.sql` to Perl source extensions globally could cause the LSP to attempt parsing non-migration SQL files (e.g., fixtures, test data, documentation).

**Mitigation**: Only include `.sql` files when they appear within migration-specific directory patterns. The `is_perl_discovery_path()` check should be `is_perl_source_path(path) || is_migration_sql_path(path)` where `is_migration_sql_path` checks for the directory context.

### Risk 2: SQL Syntax Highlighting Quality
**Problem**: The current SQL highlighting uses a simple regex (SQL_KW_RE) which won't handle complex SQL correctly.

**Mitigation**: This is acceptable for Phase 1 MVP. SQL keywords are the primary highlighting target. Full SQL parsing is out of scope for this issue.

### Risk 3: Version String Parsing
**Problem**: DeploymentHandler versions (1.001) vs sqitch versions (1.0.0) have different formats.

**Mitigation**: Use simple path-based matching for Phase 1. Navigation between files relies on directory structure, not version parsing.

---

## Task Breakdown

### Phase 1 Tasks

1. [ ] **Extend file discovery** for migration directories
   - Modify `perl-source-file/src/lib.rs` to add `.sql` extension
   - Modify `perl-workspace-index/src/discovery/mod.rs` to add migration path detection
   
2. [ ] **Add SQL syntax highlighting** for .sql migration files
   - Detect .sql file type in semantic tokens provider
   - Apply SQL keyword highlighting using existing `SQL_KW_RE` regex
   
3. [ ] **Add document links** for migration file references
   - Extend `perl-lsp-document-links/src/lib.rs` to detect migration file patterns
   - Create links between related migration files
   
4. [ ] **Update feature governance**
   - Add new feature entries in `features.toml`
   - Verify feature flags work correctly

5. [ ] **Add tests**
   - Unit tests for file discovery changes
   - Unit tests for SQL highlighting in .sql files
   - Integration tests for document links in migration contexts

---

## Verification

1. Run `cargo build -p perl-workspace-index` to verify compilation
2. Run `cargo test -p perl-workspace-index` to verify existing tests pass
3. Run `cargo test -p perl-lsp-document-links` to verify document link tests
4. Run `cargo test -p perl-lsp-semantic-tokens` to verify semantic token tests
5. Create a test Perl project with DeploymentHandler migration structure and verify:
   - .sql files in `share/deploy/` are discovered
   - .sql files are highlighted with SQL keyword coloring
   - Document links appear for migration file references

---

## Scope Boundaries

### In Scope
- File discovery for .sql files in migration directories
- SQL keyword highlighting in migration .sql files
- Document links for migration file references
- Phase 1 of the issue (file pattern recognition)

### Out of Scope
- SQL query validation or linting
- Database execution
- Version graph visualization (Phase 2)
- DeploymentHandler DSL completion (Phase 3)
- Support for non-SQL migration files
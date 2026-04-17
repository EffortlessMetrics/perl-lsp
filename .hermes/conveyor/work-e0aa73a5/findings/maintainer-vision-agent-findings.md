# Maintainer Vision Findings — work-e0aa73a5

## Alignment Assessment

**misaligned** — Phase 1 as proposed conflates three independent features (file discovery, SQL highlighting, document links) with underestimated architectural complexity, creating technical debt that will hinder the codebase's path to stability.

---

## Reasoning

### 1. The Codebase Is in Stability/Hardening Mode

From `ROADMAP.md`:
- Current version is `v0.12.4`, actively hardening the 0.12.x line
- "Now" priorities are quality cleanup, debug print removal, unused dependency removal, clippy zero-warning enforcement
- Beyond `v0.13.0`: **"Stability contract for APIs and advertised wire behavior, Performance hardening for larger workspaces, Path to v1.0.0"**

Adding support for non-Perl files (`.sql`) with specialized highlighting and document link patterns is contrary to this stability direction. It expands the surface area of the LSP without a corresponding increase in Perl developer value — SQL migration files are a tooling concern, not a language server concern for Perl specifically.

### 2. File Discovery Architecture Doesn't Support Contextual Discovery

The plan proposes adding SQL file discovery gated on migration directory context (e.g., `share/deploy/`). However:

- `is_perl_discovery_path()` (`discovery/mod.rs:64`) is a **pure filter** with signature `fn(&Path) -> bool`
- It receives **no directory context** — only the full path
- The only way to detect "migration directory context" from a `&Path` is to inspect path components, which the plan does propose
- But this is fragile: `share/deploy/1.001/001-auto.sql` works, but `db/migrations/deploy/1.001/001-auto.sql` wouldn't be discovered unless explicitly listed

The plan review correctly identified this: "The proposed mitigation cannot be implemented as described without changing the function signature or discovery pipeline architecture."

### 3. SQL Highlighting Cannot "Leverage" Existing Infrastructure

The plan states: "Leverage existing `sql_heredoc_keyword` infrastructure" for `.sql` files.

This is **architecturally incorrect**:

- `sql_heredoc_keyword` token is emitted **only** via `tokenize_sql_body()` (`semantic_tokens.rs:272`)
- `tokenize_sql_body()` is called **only** from code that detects `<<SQL` heredoc labels
- The function `heredoc_injection_language()` (`semantic_tokens.rs:254`) recognizes `<<SQL`, `<<MYSQL`, etc. and returns `Some("sql")`
- For standalone `.sql` files, this entire pipeline **never fires**

The semantic tokens documentation confirms: "Tokens are single-line only; multi-line spans emit `len = 0` and are skipped." SQL files are inherently multi-line. Naive regex application to `.sql` files would produce zero tokens.

### 4. `.sql` in `PERL_SOURCE_EXTENSIONS` Is a Semantic Misnomer

The plan proposes adding `.sql` to `PERL_SOURCE_EXTENSIONS` in `perl-source-file/src/lib.rs:37`.

From the source file comment:
> "Canonical Perl source file extensions... These helpers provide one canonical definition for what constitutes a Perl source file."

Adding `.sql` to this list is a **semantic error**: SQL is not Perl source code. This conflates two distinct file types and will confuse future maintainers. If SQL files need to be discovered, they should have a **separate discovery path**, not be lumped into "Perl source."

### 5. Document Links Extension Is New Feature Work

`compute_links()` (`document-links/lib.rs:22`) processes lines using `parse_module_import_head()` which is designed for `use`/`require` statements. The plan's suggestion to "extend" this for migration file references is actually **new detection pattern work**, not an extension of existing logic.

The verification agent confirmed: "Document Links Don't Support Inline Path Strings." This was a new finding.

---

## Impact on Codebase Trajectory

### If Merged As Proposed

1. **Technical debt accumulates**: The architectural shortcuts required (path-component-based filtering for migration context, fake "leverage" of SQL infrastructure) create debt that will need to be repaid when Phase 2 or Phase 3 arrive.

2. **SQL highlighting will be broken for months**: The "leverage existing infrastructure" approach will produce no visible SQL tokens in `.sql` files. Users will file bugs. Fixing it properly requires building a new SQL file tokenization path, which is a significant undertaking.

3. **Feature governance becomes unclear**: Adding `lsp.migration_file_discovery`, `lsp.sql_highlighting`, and `lsp.migration_document_links` to `features.toml` implies these are stable features. They're not — they're Phase 1 of a 3-phase feature that itself has unresolved architectural questions.

4. **The LSP's identity blurs**: A Perl Language Server that highlights SQL files and provides document links for migration files is less "Perl LSP" and more "generic database migration LSP." This isn't necessarily wrong, but it wasn't part of the stated roadmap.

### Six Months From Now

If Phase 1 ships with its current flaws:
- The codebase will have `.sql` in `PERL_SOURCE_EXTENSIONS` with a comment explaining it's for "migration support" — confusing
- SQL highlighting will appear broken for standalone `.sql` files
- Document links for migration files will exist but only detect simple patterns, not complex version chains
- Future work on SQL highlighting will have to untangle the "leverage heredoc infrastructure" misconception

If Phase 1 is deferred until properly designed:
- The codebase remains stable
- Phase 1 can be properly scoped with correct architectural decisions
- No misleading feature entries in `features.toml`

---

## Recommendations

### 1. Reduce Phase 1 Scope to File Discovery Only

Remove SQL highlighting and document links from Phase 1 entirely. File discovery for migration paths provides immediate value without the complex architectural implications. Per the plan review:

> "Reduce Phase 1 scope to file discovery only. Remove SQL highlighting and document links from Phase 1. File discovery for migration paths provides immediate value without the complex architectural implications."

### 2. Add a Separate Discovery Path for Migration Files

Do **not** add `.sql` to `PERL_SOURCE_EXTENSIONS`. Instead, create a **separate** discovery mechanism:

```rust
// In discovery/mod.rs
pub fn is_migration_discovery_path(path: &Path) -> bool {
    // Check path components for migration directory patterns
    // share/deploy/, share/upgrade/, share/revert/, sqitch.plan, etc.
}
```

Then discovery can call:
```rust
if is_perl_discovery_path(path) || is_migration_discovery_path(path) {
    files.push(path);
}
```

This keeps the Perl source concept clean and adds migration discovery as orthogonal.

### 3. Acknowledge SQL Highlighting Requires New Architecture

SQL highlighting for `.sql` files requires:
- A **new tokenization path** for SQL files (not reusing `tokenize_sql_body()`)
- A **file type detector** in the semantic tokens provider
- Handling of the single-line constraint for multi-line SQL

Document this as a **Phase 1b or Phase 2** task with proper architectural design. Do not claim to "leverage existing infrastructure" when the architecture explicitly prevents that reuse.

### 4. Defer Document Links for Migration Files to Phase 2

Document links between migration files are a **navigation feature**, not a file discovery feature. Move to Phase 2 where the complexity can be properly estimated.

### 5. Clarify Scope in the Issue

The issue title "feat: Schema migration support (DBIx::Class::DeploymentHandler, etc.)" spans all three phases. Phase 1 should explicitly state it only covers file discovery, not the full feature.

---

## Long-Term Impact

### Positive (if done correctly)

- Migration file discovery enables IDE support for Perl ORM workflows
- Clear separation of Perl source vs. migration files avoids semantic confusion
- Phased approach allows proper architectural design at each step

### Negative (if merged as proposed)

- `.sql` in `PERL_SOURCE_EXTENSIONS` is a semantic misnomer that will confuse future maintainers
- Broken SQL highlighting creates user-facing bugs that erode trust
- Architectural shortcuts in file discovery are fragile and will break edge cases
- Feature governance entries for incomplete features create misleading capability advertisements

### Architectural Debt Assessment

| Component | Debt Created | Difficulty to Fix |
|-----------|-------------|-------------------|
| `PERL_SOURCE_EXTENSIONS` | High (semantic confusion) | Medium (rename + separate list) |
| SQL highlighting | High (misunderstanding of architecture) | High (new tokenization path) |
| Migration file discovery | Medium (fragile path matching) | Low (path component check) |
| Document links | Low (truly new feature) | Medium (new detection pattern) |

---

## Questions the Pipeline Should Answer

1. **Is migration file discovery actually in scope for a Perl LSP?** The issue title mentions DBIx::Class::DeploymentHandler and sqitch, but these are database tools used with Perl, not Perl-specific features. Should this be a separate LSP extension rather than core perl-lsp functionality?

2. **Who are the users requesting this?** If this is a small number of users with complex migration workflows, the stability cost of adding non-Perl file support may exceed the benefit.

3. **Why not a separate SQL LSP extension?** Many editors have SQL language server extensions. Adding SQL highlighting to a Perl LSP sets a precedent for supporting every file type that appears in a Perl project.

4. **Is Phase 1 file discovery sufficient for MVP?** If the goal is to help developers find migration files in their workspace, file discovery alone achieves that. Do SQL highlighting and document links add enough value to justify the architectural complexity?

5. **What happens when a project has both DeploymentHandler AND sqitch migrations?** The plan handles these as separate path patterns, but there's no unified abstraction. Is that intentional or a gap?

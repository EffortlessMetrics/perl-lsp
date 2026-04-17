# Research Findings — work-e0aa73a5

## Issue Summary
Add IDE support for Perl database schema migration tools (DBIx::Class::DeploymentHandler, sqitch). Currently perl-lsp provides no assistance for migration file navigation, version chains, SQL syntax highlighting, or migration DSL completion.

## Relevant Codebase Areas
- `crates/perl-workspace-index/src/discovery/mod.rs` — file discovery, `is_perl_discovery_path()`
- `crates/perl-source-file/src/lib.rs` — `PERL_SOURCE_EXTENSIONS`, `is_perl_source_path()`
- `crates/perl-lsp-document-links/src/lib.rs` — `compute_links()` for document links
- `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs` — `sql_string`, `sql_heredoc_keyword`, `SQL_KW_RE`
- `crates/perl-lsp-navigation/src/lib.rs` — navigation providers
- `features.toml` — capability catalog

## Key Findings
1. **No migration-specific support exists** — codebase has no awareness of DeploymentHandler directory structures or sqitch plans
2. **File discovery is the entry point** — .sql files are currently excluded from indexing; must extend `is_perl_discovery_path()`
3. **SQL highlighting infrastructure exists** — `sql_heredoc_keyword` with `SQL_KW_RE` regex already handles SQL keywords inside Perl heredocs
4. **Document links can be extended** — `compute_links()` is a clean extension point for migration-file-to-migration-file links
5. **Three-phase scope** from issue: file pattern recognition → navigation → DSL completion

## Proposed Approach
Implement Phase 1 (file pattern recognition): extend file discovery for migration directories, add SQL syntax highlighting for .sql files, add document links for migration file references, update feature governance. This leverages existing infrastructure without modifying the Perl parser.

## Top Risks
1. **SQL file over-discovery** — adding .sql globally could index unintended files; must check directory context
2. **SQL highlighting quality** — current regex-based approach won't handle complex SQL correctly
3. **Version string parsing** — DeploymentHandler (1.001) vs sqitch (1.0.0) formats differ

## Scope
**In**: File discovery for .sql in migration dirs, SQL keyword highlighting, document links for migration files
**Out**: SQL validation, database execution, version graph visualization, DeploymentHandler DSL completion
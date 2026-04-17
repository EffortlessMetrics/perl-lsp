# Verification Findings — work-e0aa73a5

## Confidence Assessment

**Confidence: HIGH**

The research agent's analysis is well-structured and accurate. Key claims were verified against the actual codebase. However, there are minor inaccuracies worth noting (documented in Corrected Findings below).

---

## Confirmed Findings

### 1. `is_perl_discovery_path()` in `perl-workspace-index/src/discovery/mod.rs` (lines 64-73)
```rust
pub fn is_perl_discovery_path(path: &Path) -> bool {
    is_perl_source_path(path)
        || path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            ext.eq_ignore_ascii_case("i")
                || ext.eq_ignore_ascii_case("xs")
                || ext.eq_ignore_ascii_case("ep")
                || ext.eq_ignore_ascii_case("tt")
                || ext.eq_ignore_ascii_case("tt2")
        })
}
```
- **Confirmed**: `.sql` is NOT in this list. Migration-specific paths (`share/deploy/`, `share/upgrade/`, `share/revert/`, `sqitch.plan`) are NOT recognized. This matches the research agent's description.

### 2. `PERL_SOURCE_EXTENSIONS` in `crates/perl-source-file/src/lib.rs` (line 37)
```rust
pub const PERL_SOURCE_EXTENSIONS: [&str; 9] =
    ["pl", "pm", "t", "psgi", "cgi", "ep", "tt", "tt2", "mason"];
```
- **Confirmed**: `.sql` is NOT included. The research agent correctly identified this gap.

### 3. `compute_links()` in `crates/perl-lsp-document-links/src/lib.rs` (lines 22-95)
- **Confirmed**: Only handles `use` and `require` statements. Does NOT handle migration file references like `share/deploy/1.001/001-auto.sql` or `sqitch.plan`. The function signature is:
```rust
pub fn compute_links(uri: &str, text: &str, _roots: &[Url]) -> Vec<Value>
```

### 4. Semantic Tokens in `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs`
- **Confirmed**: `sql_string` (index 20, line 180) and `sql_heredoc_keyword` (index 21, line 181) both exist.
- **Confirmed**: `SQL_KW_RE` regex (lines 232-236) matches SQL keywords: SELECT, FROM, WHERE, JOIN, INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, etc.
- **Confirmed**: `heredoc_injection_language()` function (lines 254-269) recognizes `<<SQL`, `<<MYSQL`, `<<POSTGRES`, etc.
- **Confirmed**: SQL highlighting ONLY works inside Perl heredocs via `tokenize_sql_body()` function (lines 272-293). Standalone `.sql` files are NOT handled.

### 5. `perl-lsp-navigation/src/lib.rs` (line 36)
```rust
pub use perl_lsp_document_links::compute_links;
```
- **Confirmed**: The navigation crate re-exports `compute_links` from `perl_lsp_document_links`.

### 6. `perl-lsp-completion` crate
- **Confirmed**: The crate exists at `crates/perl-lsp-completion/` (not `perl-lsp-completion/src/`). The completion subsystem provides context-aware code completion with providers for builtins, variables, functions, methods, packages, workspace, file_path, etc.

### 7. `features.toml` entries
- **Confirmed**: `lsp.document_link` (id 157, line 157) exists with description referencing "Document links to modules and docs".
- **Confirmed**: `lsp.document_link_resolve` (id 901, line 901) exists.
- **Confirmed**: `lsp.semantic_tokens` (id 184, line 184) advertises 23 token types including `sql_string`, `sql_heredoc_keyword`, and `json_heredoc_key`.

### 8. No Migration-Specific Support Exists
- **Confirmed via search**: No occurrences of "DeploymentHandler", "sqitch", or "migration.*sql" patterns exist anywhere in the codebase.

---

## Corrected Findings

### 1. Completion Crate Path
**Research Agent**: `crates/perl-lsp-completion/src/` (completion/, completion.rs, lib.rs)
**Actual**: The crate is `crates/perl-lsp-completion/` and the path `src/` contains `completion/`, `completion.rs`, and `lib.rs`. The slash notation was ambiguous but the content description is correct.

### 2. Issue URL Domain
**Research Agent**: References `github.com/EffortlessMetrics/perl-lsp/issues/3564`
**Actual**: The local repo is at `/home/hermes/repos/perl-lsp` (no evidence of this being EffortlessMetrics). The issue URL may be a placeholder or a fork reference. This is a minor issue naming discrepancy, not a factual error about the codebase.

### 3. Semantic Token Index Values
**Research Agent**: States "Token types 0-19 are standard LSP; 20-22 are Perl-specific extensions"
**Correction**: Standard LSP tokens are 0-7 in the specification, but this codebase extends to 22 total types. The exact boundary of "standard" vs "Perl-specific" isn't 19 - the research agent's breakdown oversimplifies but doesn't materially affect the implementation approach.

---

## New Findings

### 1. File Discovery Architecture Has Two Entry Points
The `is_perl_discovery_path()` function is the gatekeeper, but there are TWO discovery strategies using it:
- `try_git_discovery()` (line 75) - uses `git ls-files` then filters with `is_perl_discovery_path()`
- `walk_discovery()` (line 126) - uses WalkDir then filters with `is_perl_discovery_path()`

The research agent correctly identified the `is_perl_discovery_path()` function as the entry point but didn't note that BOTH discovery paths call it as a filter.

### 2. The `path_contains_skipped_component()` Skip List
The discovery module uses `path_contains_skipped_component()` to skip directories like `.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`, and `blib`. If migration directories like `share/` need to be discovered, they must NOT be added to this skip list. This is a potential friction point the research agent didn't anticipate.

### 3. `perl-workspace-index` Has Its Own `ignore` Module
The discovery module uses `crate::ignore::{is_skipped_dir_name, path_contains_skipped_component}`. This internal ignore module controls directory traversal and could conflict with migration-specific path detection if not properly extended.

### 4. Semantic Tokens Are Single-Line Only
From the `perl-lsp-semantic-tokens/CLAUDE.md`: "Tokens are single-line only; multi-line spans emit `len = 0` and are skipped." This is an architectural constraint that affects how SQL keyword highlighting in `.sql` files would need to work - it requires the file to be parsed and tokenized by this crate, not just regex-matched.

### 5. SQL Keyword Highlighting Requires Identifying File Type First
The `SQL_KW_RE` regex and `tokenize_sql_body()` function exist, but they only fire inside heredoc bodies via `heredoc_injection_language()`. For standalone `.sql` files, the semantic tokens provider would need to:
1. Detect the file is `.sql` or in a migration directory
2. Apply SQL keyword highlighting without the heredoc injection mechanism

The research agent suggested "leveraging existing `sql_heredoc_keyword` infrastructure" but this infrastructure is specifically tied to heredoc recognition, not standalone file support.

### 6. Document Links Don't Support Inline Path Strings
The `compute_links()` function processes entire lines with `parse_module_import_head()` for `use`/`require` statements. It has NO mechanism for detecting inline migration file paths like `"share/deploy/1.001/001-auto.sql"` within a string. This would require a new detection pattern entirely.

---

## Scope Assessment

**Issue Title**: "feat: Schema migration support (DBIx::Class::DeploymentHandler, etc.)"

**Actual Scope**: Phase 1 focuses on file pattern recognition for migration files, which touches:
1. `crates/perl-source-file/src/lib.rs` - PERL_SOURCE_EXTENSIONS
2. `crates/perl-workspace-index/src/discovery/mod.rs` - is_perl_discovery_path()
3. `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs` - SQL highlighting for .sql files
4. `crates/perl-lsp-document-links/src/lib.rs` - migration file link detection
5. `features.toml` - feature catalog updates

**Scope Mismatch**: The issue title mentions "DBIx::Class::DeploymentHandler, etc." which implies Phase 2 (navigation) and Phase 3 (DSL completion) are in scope eventually, but the plan only covers Phase 1. This is appropriate for a first implementation but the issue title is broader than Phase 1.

---

## Verification Methodology

### Commands Run
1. `grep` patterns for SQL-related tokens in semantic-tokens crate
2. `ls` of crate directories to verify existence
3. Direct file reads of key source files with line number verification
4. `features.toml` scanning for feature IDs
5. Search for "DeploymentHandler", "sqitch", "migration" patterns in codebase

### Files Verified (with line numbers)
| File | Lines Verified | Key Finding |
|------|---------------|-------------|
| `crates/perl-workspace-index/src/discovery/mod.rs` | 64-73 | `is_perl_discovery_path()` structure confirmed |
| `crates/perl-source-file/src/lib.rs` | 37-38 | `PERL_SOURCE_EXTENSIONS` confirmed as 9-element array |
| `crates/perl-lsp-document-links/src/lib.rs` | 22-95 | `compute_links()` only handles use/require |
| `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs` | 180-182, 232-293 | SQL tokens and regex confirmed |
| `crates/perl-lsp-navigation/src/lib.rs` | 36 | Re-export confirmed |
| `crates/perl-lsp-completion/` | dir listing | Crate exists confirmed |
| `features.toml` | 157, 901 | Feature IDs confirmed |

### Confidence Justification
The research agent provided specific file paths, line numbers, and code snippets. I verified each claim by reading the actual source files. The analysis is accurate and the implementation approach in the plan is consistent with the codebase architecture.

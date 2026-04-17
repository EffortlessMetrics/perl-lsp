# Plan Review Findings — work-e0aa73a5

## Overall Assessment

**feasible with modifications** — Phase 1 file pattern recognition is achievable, but three major technical assumptions are wrong: (1) SQL highlighting infrastructure cannot be "leveraged" for standalone .sql files without new tokenization logic, (2) the SQL over-discovery mitigation requires architectural changes to discovery filtering, and (3) document links for migration files is a new feature, not an extension.

## Scope Assessment

**Scope title mismatch**. The issue title "feat: Schema migration support (DBIx::Class::DeploymentHandler, etc.)" implies all three phases (file recognition, navigation, completion) are in scope, but the plan only covers Phase 1. This is appropriate for an initial implementation but creates ambiguity about what constitutes "done." The plan should explicitly state that Phase 1 scope is limited to file pattern recognition and acknowledge Phases 2 and 3 are future work.

## What Works

1. **Phase 1 is the correct starting point**. File pattern recognition is the safest first step because it doesn't require modifying the Perl parser or semantic analyzer.

2. **Verification of codebase gaps is accurate**. The research and verification agents correctly identified that `.sql` is absent from `PERL_SOURCE_EXTENSIONS`, migration paths are unrecognized, and SQL highlighting only works in heredocs.

3. **Risk identification is solid**. The plan correctly identifies SQL over-discovery, highlighting quality, and version string parsing as risks.

4. **Task breakdown is concrete**. Each task names specific files with specific modifications, making the scope traceable.

5. **Verification steps are testable**. The cargo build/test commands provide immediate feedback loops.

## What Doesn't Work

### 1. SQL Highlighting Cannot Leverage Existing Infrastructure for .sql Files

**Problem**: The plan states "Leverage existing `sql_heredoc_keyword` infrastructure" to add SQL highlighting for `.sql` files. This is technically incorrect.

**Evidence**: `tokenize_sql_body()` (semantic_tokens.rs:272-293) only processes text passed to it via `heredoc_injection_language()` (lines 254-269). This function recognizes `<<SQL`, `<<MYSQL`, etc. and only then calls `tokenize_sql_body()` on the heredoc body. Standalone `.sql` files never pass through this path — the entire file tokenization flow is different.

**Impact**: The plan underestimates this task by claiming it's an extension when it's actually building a new tokenization pathway for .sql files.

**Fix needed**: The plan must acknowledge that SQL highlighting for standalone .sql files requires:
- Detecting `.sql` file type in the semantic tokens provider
- Creating a separate SQL file tokenization path (not reusing `tokenize_sql_body()`)
- Using `SQL_KW_RE` regex to highlight keywords in that context

### 2. SQL Over-Discovery Mitigation Requires Architectural Change

**Problem**: The plan's mitigation for SQL over-discovery is to "only include `.sql` files when they appear within migration-specific directory patterns." The plan says `is_perl_discovery_path()` should be `is_perl_source_path(path) || is_migration_sql_path(path)`.

**Evidence**: `is_perl_discovery_path()` receives only a `&Path` (discovery/mod.rs:64). It has no access to directory context beyond what the path itself reveals. Both discovery paths (`parse_git_ls_files_output()` at line 99 and `walk_discovery()` at line 126) call `is_perl_discovery_path()` as a simple filter with no additional context.

**Impact**: The proposed mitigation cannot be implemented as described without changing the function signature or discovery pipeline architecture.

**Fix needed**: The plan must specify how directory context will be passed to the discovery filter. Options:
- Add a `is_migration_sql_path(path: &Path) -> bool` helper that checks path components for `share/deploy/`, `share/upgrade/`, etc. (works within the current architecture but is fragile)
- Restructure the discovery pipeline to pass parent directory context
- Accept the risk and include `.sql` globally with a note that Phase 2 will add filtering

### 3. Document Links for Migration Files Is a New Feature, Not an Extension

**Problem**: The plan says "Add detection for migration file references within Perl files" by "extend `perl-lsp-document-links/src/lib.rs`."

**Evidence**: `compute_links()` (document-links/lib.rs:22-95) processes lines via `parse_module_import_head()` for `use`/`require` statements. It has no mechanism for detecting inline path strings like `"share/deploy/1.001/001-auto.sql"`. The verification agent confirmed this: "Document Links Don't Support Inline Path Strings."

**Impact**: The plan implies this is a modification, but it's actually building a new detection pattern from scratch.

**Fix needed**: The plan should explicitly state this requires a new detection pass within `compute_links()` or a new function, and estimate the complexity accordingly.

### 4. Semantic Tokens Single-Line Constraint Affects .sql Files

**Problem**: The semantic tokens crate documentation states "Tokens are single-line only; multi-line spans emit `len = 0` and are skipped."

**Evidence**: In `tokenize_sql_body()` (line 288): `let len = if sl == el { ec.saturating_sub(sc) } else { 0 };` — multi-line matches produce `len = 0` and are skipped.

**Impact**: SQL files are inherently multi-line. A naive application of SQL keyword regex to a full .sql file would match multi-line keywords and those matches would be discarded.

**Fix needed**: The plan should specify that SQL file tokenization must handle multi-line SQL by emitting one token per line, matching the existing single-line constraint.

### 5. Feature Governance Entries Are Unspecified

**Problem**: The plan says to add feature entries in `features.toml` for `lsp.migration_file_discovery`, `lsp.sql_highlighting`, and `lsp.migration_document_links`, but provides no details on:
- Which features already exist (e.g., `lsp.semantic_tokens` already advertises SQL token types)
- Whether new feature IDs are needed or existing ones extended
- The mechanism for feature flag verification

**Fix needed**: The plan should list each feature.toml entry with its current state and proposed change.

## Top Risks

### Risk 1: SQL File Tokenization Architecture Is Underestimated
- **Likelihood**: high
- **Impact**: Phase 1 SQL highlighting task is significantly more complex than estimated; could consume most of Phase 1 sprint time
- **Mitigation**: Break SQL highlighting into its own task with a separate design. Consider whether SQL highlighting for .sql files is actually needed for Phase 1 MVP (file discovery alone may provide sufficient value).

### Risk 2: Discovery Pipeline Architecture Change Required for Safe .sql Filtering
- **Likelihood**: high
- **Impact**: The proposed `is_migration_sql_path()` mitigation cannot work as described; without architectural changes, either all .sql files are discovered (over-breadth) or none are (under-breadth)
- **Mitigation**: Add `is_migration_sql_path(path: &Path) -> bool` that checks path components for migration-specific directory names, accepting the fragility, OR defer SQL file discovery until a more robust solution is designed.

### Risk 3: `share/` Directory May Conflict with Skip List
- **Likelihood**: medium
- **Impact**: If `share/` is added to the skip list by `path_contains_skipped_component()`, migration files would never be discovered even with correct path filtering
- **Mitigation**: Verify `share/` is not in the skip list before implementing migration path detection. Check `should_skip_dir()` and `path_contains_skipped_component()` implementations.

### Risk 4: sqitch.plan Parsing Is Out of Scope But Required for Full Support
- **Likelihood**: low
- **Impact**: Phase 1 discovery includes `sqitch.plan` as a recognized file, but the plan file format is complex and not parseable with simple path matching
- **Mitigation**: Treat `sqitch.plan` discovery as a stub for future work; don't attempt to parse the file in Phase 1.

## Edge Cases

1. **SQL files in non-migration directories**: `./sql/`, `./scripts/`, `./db/migrations/` would not be discovered even if they contain migration-related SQL.

2. **SQL files with non-standard extensions**: `.mysql`, `.pgsql`, `.sqlite` would not be discovered.

3. **sqitch.plan in nested directories**: `db/core/sqitch.plan` would not be discovered with simple filename matching.

4. **Mixed migration tool usage**: A project using both DeploymentHandler and sqitch would need both path patterns recognized.

5. **Case sensitivity**: DeploymentHandler uses `share/deploy/` (lowercase) but some projects may use `Share/Deploy/` on case-insensitive filesystems.

6. **Very large .sql files**: SQL migration files can be hundreds of KB; single-line tokenization could produce thousands of tokens.

7. **Non-UTF8 SQL files**: Discovery would index them but syntax highlighting may fail.

## Recommendations

1. **Reduce Phase 1 scope to file discovery only**. Remove SQL highlighting and document links from Phase 1. File discovery for migration paths provides immediate value without the complex architectural implications.

2. **Add `is_migration_sql_path()` function with path component checking**. Implement a helper that checks if any path component matches known migration directory names (`deploy`, `upgrade`, `revert`, `verify`, `sqitch.plan`). Accept this is fragile but workable for Phase 1.

3. **Separate SQL highlighting into Phase 1b or Phase 2**. Acknowledge that SQL highlighting for .sql files requires new tokenization logic and is a separate feature from heredoc SQL highlighting.

4. **Move document links for migration files to Phase 2**. This is a navigation feature, not a file discovery feature.

5. **Verify skip list before implementing**. Run a check to confirm `share/` and migration directory names are not in the skip list.

6. **Add concrete feature.toml entries**. List each feature by name, ID, current state, and proposed change.

7. **Specify verification methodology for file discovery**. The current verification step ("create a test Perl project") is manual. Specify automated test cases for `is_perl_discovery_path()` with migration paths.

## Confidence to Proceed

**medium** — The plan's structure and task breakdown are solid, but three core technical assumptions are wrong: SQL highlighting cannot reuse heredoc infrastructure, the SQL over-discovery mitigation requires architectural changes, and document links is a new feature not an extension. These issues are fixable without redesign, but the plan needs revision before proceeding to DESIGNED.

**What would raise confidence:**
- Confirmation that `is_migration_sql_path(path: &Path)` can work by checking path components (not just filename)
- Acceptance that SQL highlighting for .sql files is out of scope for Phase 1 (or a separate Phase 1b)
- Explicit scoping of document links as a Phase 2 navigation feature, not Phase 1
- Verification that `share/` is not in the skip list
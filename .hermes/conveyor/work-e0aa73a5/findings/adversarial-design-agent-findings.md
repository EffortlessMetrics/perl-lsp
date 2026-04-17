# Adversarial Design Findings — work-e0aa73a5

## Current Approach

The plan proposes to implement **Phase 1** of the schema migration feature: file pattern recognition for DBIx::Class::DeploymentHandler and sqitch migration files. Specifically:

1. Add `.sql` to `PERL_SOURCE_EXTENSIONS` in `perl-source-file`
2. Extend `is_perl_discovery_path()` to recognize migration-specific directories (`share/deploy/`, `share/upgrade/`, `share/revert/`, sqitch paths) and `.sql` files within them
3. Apply SQL keyword highlighting via the existing `SQL_KW_RE` regex when `.sql` files are opened
4. Add document links for migration file references in Perl code
5. Update `features.toml` with new capability entries

The rationale is that Phase 1 is "safest" because it doesn't require modifying the Perl parser/semantic analyzer, leverages existing infrastructure, and provides immediate value.

---

## Alternative Approaches

### Alternative 1: Workspace Index Only (No LSP Features for `.sql` Files)

**Core idea:** Extend workspace indexing to discover migration files, but do **not** attempt to parse `.sql` as Perl or provide semantic highlighting for them. Let the editor's native SQL support (VS Code has built-in SQL highlighting) or existing SQL extensions handle highlighting. The Perl LSP focuses purely on enabling Perl code to reference and navigate to migration files.

**Why it might be better:**
- Zero technical risk — we never try to parse SQL as Perl
- No semantic lies — `.sql` files are never incorrectly classified as "Perl source"
- The Perl LSP's core competency (Perl analysis) stays sharp
- Users with dedicated SQL extensions get better SQL support than we could provide
- Migration file discovery still enables `Go to Definition` for path references in Perl code
- Document links from Perl → migration files still work via path detection

**Why it might be worse:**
- SQL files opened directly in the editor don't get Perl-adjacent features from this LSP
- No "migration file to migration file" navigation (e.g., jumping from `deploy/1.001/001.sql` to `upgrade/1.001-1.002/001.sql`)
- The issue explicitly requests SQL highlighting, so this is partially incomplete

**What it sacrifices:** The SQL highlighting feature requested in the issue. However, SQL highlighting is available natively in all major editors without our involvement.

---

### Alternative 2: Strict Architecture — SQL Document Type With Separate Pipeline

**Core idea:** When a `.sql` file is opened, treat it as a **foreign document type** — not Perl. Rather than trying to run the Perl parser on it, create a lightweight SQL-specific handling path: parse-free document links via regex on path patterns, and a separate SQL tokenizer (not dependent on Perl AST) for keyword highlighting.

**Why it might be better:**
- Architecturally honest: SQL files are not Perl files, and the code reflects that
- SQL highlighting works correctly because it uses a dedicated SQL tokenizer
- The Perl parser never sees `.sql` input — no garbage AST produced
- Migration-to-migration links work correctly
- The `perl-source-file` crate's semantics stay clean (`.sql` is never a "Perl source extension")

**Why it might be worse:**
- Significant engineering effort — a separate document handling path
- Maintenance burden: two highlighting pipelines instead of one
- Adds complexity to a codebase that values simplicity
- The `features.toml` capability entries become more complex

**What it sacrifices:** Simplicity. The current plan is straightforward because it reuses Perl infrastructure everywhere. This alternative introduces a second infrastructure.

---

### Alternative 3: Delegation to Editor SQL Extensions via LSP Metadata

**Core idea:** Instead of implementing SQL highlighting ourselves, advertise in the LSP `initialize` response that `.sql` files are supported via a different language server (via `textDocument.languageServerWorkspaceLibrary` or similar mechanism), and focus exclusively on:
1. Making migration directories discoverable by the workspace index
2. Making Perl code's path references to migration files clickable via document links
3. Providing a command or navigation provider to jump between related migration files

**Why it might be better:**
- The Perl LSP doesn't need to become a mediocre SQL IDE
- Users with established SQL tooling (SQLTools, dbml, etc.) get their preferred experience
- The Perl LSP stays focused and maintainable
- Migration discovery + Perl-to-migration navigation are genuinely useful and unique to this context
- No duplication of effort — we don't reimplement SQL highlighting

**Why it might be worse:**
- Requires discovering how to properly delegate to other language servers via LSP (may not be standardized)
- No direct SQL highlighting from this LSP
- "Migration file to migration file" navigation still missing unless we implement it

**What it sacrifices:** Full in-LSP SQL highlighting. But this is arguably not the Perl LSP's job.

---

## Strongest Argument Against Current Approach

The current plan's SQL highlighting strategy is **architecturally impossible** with the existing pipeline.

From `crates/perl-lsp/src/runtime/language/semantic_tokens.rs` lines 32-48:

```rust
if let Some(ref ast) = doc.ast {
    let data = crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, ...);
    return Ok(Some(json!({ "data": flat_data })));
}
```

The semantic tokens handler **requires `doc.ast`** — a parsed Perl AST. When a `.sql` file is opened:

1. `text_sync.rs` line 167-182: `Parser::new(code_text)` attempts to parse `.sql` content as **Perl** — producing a garbage/failed AST
2. The semantic tokens handler then runs `collect_semantic_tokens()` on this garbage AST
3. Even if the AST exists, `collect_semantic_tokens()` uses `PerlLexer` — a Perl tokenizer — on SQL content

The plan's claim that "SQL highlighting only works inside Perl heredocs, NOT in standalone .sql files" (from research_analysis line 49) is correct — but the proposed fix (extending to standalone `.sql` files via the same pipeline) **fundamentally cannot work** because the pipeline requires a Perl AST that won't exist for SQL.

The mitigation in the plan's Risk #1 ("Only include `.sql` files when they appear within migration-specific directory patterns") does not fix this — the problem is not *which* `.sql` files get processed, it's that the ** Perl semantic token pipeline** cannot produce meaningful tokens for **any** non-Perl content.

---

## Recommended Action

**Modify the current approach** with the following specific changes:

1. **Remove `.sql` from `PERL_SOURCE_EXTENSIONS`** — this is a semantic error. SQL is not Perl source. Use a separate `is_migration_sql_path()` check that looks at directory context only.

2. **Separate the pipeline** for `.sql` files:
   - For `.sql` files in migration directories, **skip Perl AST parsing entirely**
   - Provide SQL highlighting via a **standalone SQL tokenizer** that does not depend on `doc.ast`
   - The existing `SQL_KW_RE` regex can be reused, but outside the Perl semantic token pipeline

3. **Change the semantic tokens handler** to detect `.sql` files and return SQL tokens without requiring `doc.ast`:
   - Add a path/extension check in `handle_semantic_tokens()` to detect `.sql` files
   - For `.sql` files, run `SQL_KW_RE` directly on `doc.text` and emit tokens
   - This avoids the broken Perl AST → semantic token path

4. **Start with Alternative 1** as the scope for Phase 1: workspace discovery + Perl-to-migration document links. SQL highlighting for `.sql` files (when not in Perl context) is Phase 1B if implemented via a separate pipeline, otherwise defer to Phase 2.

---

## Long-Term Cost Assessment

**If we do it the current way (with the architectural fixes above, not the original plan):**

- **6 months:** The separate SQL pipeline adds maintenance burden. Every change to token types needs to consider both Perl and SQL paths.
- **2 years:** The dual-pipeline approach becomes technical debt. New engineers must understand both. Feature flags multiply (`lsp.sql_highlighting`, `lsp.migration_file_discovery`, etc.).

**If we do Alternative 1 (workspace index only):**

- **6 months:** Minimal change, easily understood. No new maintenance burden.
- **2 years:** The workspace index gains migration file awareness. Perl-to-migration navigation works. SQL highlighting remains delegated to the editor. This is stable and low-cost.

**If we do Alternative 2 (separate SQL pipeline):**

- **6 months:** Significant engineering investment. Must build and test a separate SQL document handler.
- **2 years:** Clean architecture pays off. SQL and Perl concerns are properly separated. But only if the SQL pipeline is well-designed and doesn't drift from the Perl pipeline's patterns.

**Bottom line:** The current approach (even modified) introduces SQL as a second-class concern into a Perl-centric codebase. Alternative 1 is the cheapest approach that satisfies the core need (migration file discovery and Perl-to-migration navigation). Alternative 2 is the most architecturally sound if we have the bandwidth. The original plan's approach is unworkable without the architectural fixes identified above.

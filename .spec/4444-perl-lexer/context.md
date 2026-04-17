# Context: Wave C Microcrate Collapse (lexer satellites -> perl-lexer)

Issue: #4444 | Tracking: #4410 | ADR: [docs/adr/0041-microcrate-collapse.md](../../docs/adr/0041-microcrate-collapse.md) | Amendment: #4446 (perl-token stays published)

Pilots: #4422 (Wave 1 perl-module) | #4434 (Wave A perl-workspace) | #4438 (Wave B perl-symbol)

---

## Overview

Wave C of ADR-0041 (microcrate collapse). Absorbs **4** `perl-lexer`-adjacent satellite crates into the existing published `perl-lexer` facade. `perl-lexer` is one of the 5 foundation primitives that remain published (per ADR Amendment 1).

**Target:** 4 crates absorbed. `perl-lexer` retains its name, version, and published status. `perl-token` stays published separately (ADR Amendment 4 / PR #4446).

### Absorbed crates

| Crate | Source files | Test files | Dependencies |
|-------|-------------|------------|--------------|
| `perl-tokenizer` | 5 (`lib.rs`, `token_stream.rs`, `token_wrapper.rs`, `trivia.rs`, `trivia_parser.rs`, `util.rs`; +`trivia_parser.rs.backup` orphan) | 6 | perl-lexer, perl-token, perl-error, perl-position-tracking, perl-ast-v2 |
| `perl-keywords` | 1 (`lib.rs`) | 4 | none |
| `perl-builtins` | 2 (`lib.rs`, `builtin_signatures.rs`) | 3 | perl-builtins-phf |
| `perl-builtins-phf` | 1 (`lib.rs`) | 1 | phf (external) |

**Total test files absorbed:** 14 (6 + 4 + 3 + 1).

### NOT absorbed (per amendment #4446)

- `perl-token` -- foundation primitive (ADR Amendment 4); stays published with its own crate, consumed downstream of the lexer.

---

## Decision Log

### 1. Crate target: existing `perl-lexer` (no rename, no new crate)

- **Decision:** Absorb 4 satellites into the **existing** `crates/perl-lexer/` directory. Do not rename or create a new crate.
- **Rationale:** `perl-lexer` is already the published foundation primitive for lexing; the absorbed satellites all orbit around it. Keeping the name preserves public identity across the v0.13.0/v0.14.x line.
- **Contrast with Wave B:** Wave B created a new `perl-symbol` directory because none of the 4 satellites was a natural owner; Wave C has an obvious owner.

### 2. Layout: flat module folders inside `crates/perl-lexer/src/`

- **Decision:** Each absorbed crate becomes a sibling folder under `src/`:
  - `src/keywords/mod.rs` (from perl-keywords)
  - `src/builtins/mod.rs` + `src/builtins/phf_lookup.rs` (from perl-builtins + perl-builtins-phf)
  - `src/tokenizer/{mod.rs,token_stream.rs,token_wrapper.rs,util.rs}` (AST-agnostic slice of perl-tokenizer)
- **Rationale:** Proven pattern from Wave 1, Wave A, Wave B. Flat is simpler than nested.
- **PHF as file, not folder:** `perl-builtins-phf` becomes `src/builtins/phf_lookup.rs` (a single file inside `builtins/`), not its own folder. Per oppositional-planner R3 and architecture review: PHF is a data-shape artifact, not a module boundary.
- **Existing files untouched:** `src/token.rs` (defines `perl_lexer::{Token, TokenType, StringPart}`) stays as-is because `perl-token` is NOT absorbed. No collision.

### 3. Tokenizer split: AST-agnostic slice only

**This is the critical architectural constraint of Wave C.**

- **Decision:** Absorb ONLY `token_stream.rs`, `token_wrapper.rs`, and `util.rs` from perl-tokenizer into `perl-lexer/src/tokenizer/`.
- **Defer trivia modules:** `trivia.rs` and `trivia_parser.rs` depend on `perl-ast-v2`. Moving them into `perl-lexer` would add `perl-ast-v2` as a dependency of a foundation primitive crate -- an inversion.
- **Trivia destination:** Move `trivia.rs` and `trivia_parser.rs` into `crates/perl-parser-core/src/tokens/` (as `trivia.rs` and `trivia_parser.rs` sibling modules). `perl-parser-core` already depends on `perl-ast-v2` and already re-exports these modules at `perl_parser_core::{trivia, trivia_parser}`. The public re-export path stays unchanged for all consumers (architecture-reviewer ALIGNED).
- **Orphan file:** Delete `crates/perl-tokenizer/src/trivia_parser.rs.backup` (stale backup, not in any module tree).

### 4. `api.rs` facade: explicit re-exports, NO wildcards

- **Decision:** Create `crates/perl-lexer/src/api.rs` with explicit named `pub use` statements for public items from each absorbed module. No `pub use <module>::*;`.
- **Lib.rs pattern:** Append `pub mod api; pub use api::*;` to existing `src/lib.rs`. Existing `pub use` re-exports (`Token`, `TokenType`, `StringPart`, `Checkpointable`, `LexerMode`, etc.) stay where they are -- they are not from absorbed modules.
- **Items to re-export via api.rs:**
  - From `keywords`: 7 consts (`KEYWORDS`, `LSP_COMPLETION_KEYWORDS`, `DAP_COMPLETION_KEYWORDS`, `LSP_RUNTIME_COMPLETION_KEYWORDS`, `RENAME_KEYWORDS`, `PARSER_LSP_KEYWORDS`, `LEXER_KEYWORDS`) + 7 fns (`is_keyword`, `is_lexer_keyword`, `is_lsp_completion_keyword`, `is_dap_completion_keyword`, `is_lsp_runtime_completion_keyword`, `is_rename_keyword`, `is_parser_lsp_keyword`)
  - From `builtins::builtin_signatures`: `BuiltinSignature` struct + `create_builtin_signatures` fn
  - From `builtins::phf_lookup`: `BUILTIN_SIGS`, `BUILTIN_FULL_SIGS`, `get_param_names`, `is_builtin`, `builtin_count`
  - From `tokenizer::token_stream`: `TokenStream` (note: re-exports `perl_token::{Token, TokenKind}`; only `TokenStream` itself is new)
  - From `tokenizer::token_wrapper`: `TokenWithPosition`, `PositionTracker`
  - From `tokenizer::util`: `find_data_marker_byte_lexed`, `code_slice`, (deprecated) `find_data_marker_byte`
- **Rationale:** Explicit re-exports document public surface; wildcards allow silent API expansion. Wave 1 / Wave B pattern.

### 5. Cargo deps for the absorbing `perl-lexer` crate

- **Add** to `[dependencies]`: `phf = { version = "0.13.1", features = ["macros"] }` (was direct dep of perl-builtins-phf).
- **No change needed** for `perl-keywords`, `perl-tokenizer`, `perl-builtins`, `perl-builtins-phf` deps -- they disappear when the satellite dirs are deleted.
- **Already present:** `unicode-ident`, `memchr`, `tracing`, `thiserror`, `perl-position-tracking`, `perl-keywords` (the latter self-referential and removed). `perl-lexer` currently has `perl-keywords = { workspace = true }` at line 26 -- remove it (keywords becomes an internal module, not a separate dep).
- **NOT added:** `perl-ast-v2`, `perl-error`, `perl-token` -- those are trivia-module deps that stay with the moved trivia modules in `perl-parser-core` (decision 3).
- **Existing dev-deps stay:** `criterion`, `proptest`.

### 6. Consumer crate migrations (9 crates)

Consumers must migrate import paths. The table below lists *every* file that needs a change (expanded from scout's 5-crate list by oppositional-planner verification).

| Consumer | Cargo.toml dep change | Source file changes |
|----------|----------------------|---------------------|
| `perl-dap` | remove `perl-keywords` | `src/debug_adapter/mod.rs:43` `use perl_keywords::DAP_COMPLETION_KEYWORDS;` -> `use perl_lexer::DAP_COMPLETION_KEYWORDS;` |
| `perl-lsp` | remove `perl-keywords` | `src/runtime/language/completion.rs:21` `perl_keywords::LSP_RUNTIME_COMPLETION_KEYWORDS` -> `perl_lexer::LSP_RUNTIME_COMPLETION_KEYWORDS` |
| `perl-lsp-code-actions` | remove `perl-builtins` | `src/enhanced/import_management.rs:92` `use perl_builtins::builtin_signatures_phf::is_builtin;` -> `use perl_lexer::is_builtin;` |
| `perl-lsp-completion` | remove `perl-keywords` | `src/completion/keywords.rs:11` `use perl_keywords::LSP_COMPLETION_KEYWORDS;` -> `use perl_lexer::LSP_COMPLETION_KEYWORDS;` |
| `perl-lsp-inlay-hints` | remove `perl-builtins` | `src/inlay_hints.rs:21` `use perl_builtins::builtin_signatures::create_builtin_signatures;` -> `use perl_lexer::create_builtin_signatures;` |
| `perl-lsp-rename` | remove `perl-keywords` | `src/rename/validate.rs:5` `use perl_keywords::is_rename_keyword;` -> `use perl_lexer::is_rename_keyword;` |
| `perl-parser` | remove `perl-keywords` | `src/ide/lsp_compat/rename.rs:59` and `src/ide/lsp_compat/completion.rs:64` -- rewrite `perl_keywords::` to `perl_lexer::` |
| `perl-parser-core` | remove `perl-tokenizer` + `perl-builtins`; KEEP `perl-ast-v2` (needed for moved trivia). `perl-lexer` already present. | `src/lib.rs:89` `pub use perl_builtins as builtins;` -> `pub use perl_lexer as builtins;` (or drop entire re-export -- see note). `src/lib.rs:114` `pub use perl_tokenizer::util;` -> `pub use perl_lexer::tokenizer::util;`. `src/tokens/mod.rs:18-22` re-exports: `token_wrapper` becomes `perl_lexer::tokenizer::token_wrapper`; `trivia` and `trivia_parser` become local `pub mod trivia; pub mod trivia_parser;` (moved in from tokenizer). `src/tokens/token_stream.rs:27` `pub use perl_tokenizer::token_stream::*;` -> `pub use perl_lexer::tokenizer::token_stream::*;` |
| `perl-lexer` itself | register 4 new modules in `lib.rs`; add `api` module | update `use perl_keywords::is_lexer_keyword;` on line 139 to `use crate::keywords::is_lexer_keyword;` |

**Note on `perl-parser-core::builtins` re-export:** `pub use perl_builtins as builtins;` exposes a module name `builtins` on parser-core. With `perl-lexer` absorbing `perl-builtins`, the equivalent is `pub use perl_lexer::builtins;` (the `builtins` module is pub inside lib.rs). Preserve the public alias to avoid breaking `perl_parser_core::builtins::builtin_signatures_phf` callers.

### 7. Test file migration and prefix scheme (Wave 1 pattern)

14 test files migrate to `crates/perl-lexer/tests/` with crate-source prefixes to resolve collisions. The existing `perl-lexer/tests/` directory has 19 test files already (see below); new tests get distinguishing prefixes.

**From `perl-tokenizer/tests/` (6 files):**

| Source | Destination |
|--------|-------------|
| `comprehensive_unit_tests.rs` | `tokenizer_comprehensive_unit_tests.rs` (COLLISION with existing lexer file? no -- existing lexer has `comprehensive_unit_tests.rs` too -- must prefix) |
| `extended_unit_tests.rs` | `tokenizer_extended_unit_tests.rs` |
| `edge_case_tests.rs` | `tokenizer_edge_case_tests.rs` (COLLISION with existing `edge_case_tests.rs`) |
| `bdd_consolidated.rs` (file is `tokenizer_bdd_consolidated.rs` -- already prefixed) | `tokenizer_bdd_consolidated.rs` (keep as-is) |
| `bridge_coverage_tests.rs` | `tokenizer_bridge_coverage_tests.rs` |
| `trivia_edge_cases.rs` | **SKIP** -- trivia logic moves to `perl-parser-core`; this test follows (see decision 8 below) |

**From `perl-keywords/tests/` (4 files):**

| Source | Destination |
|--------|-------------|
| `comprehensive_unit_tests.rs` | `keywords_comprehensive_unit_tests.rs` (COLLISION) |
| `keyword_classification.rs` | `keywords_classification.rs` (redundant-sounding but avoids mystery prefix) |
| `keyword_edge_cases.rs` | `keywords_edge_cases.rs` |
| `bdd_keyword_qw.rs` | `keywords_bdd_qw.rs` |

**From `perl-builtins/tests/` (3 files):**

| Source | Destination |
|--------|-------------|
| `comprehensive_unit_tests.rs` | `builtins_comprehensive_unit_tests.rs` (COLLISION) |
| `extended_unit_tests.rs` | `builtins_extended_unit_tests.rs` |
| `bdd_scenarios.rs` | `builtins_bdd_scenarios.rs` |

**From `perl-builtins-phf/tests/` (1 file):**

| Source | Destination |
|--------|-------------|
| `comprehensive_unit_tests.rs` | `builtins_phf_comprehensive_unit_tests.rs` |

**Import rewrites per-test file:**
- `use perl_keywords::X;` -> `use perl_lexer::X;` (via api.rs re-exports at crate root)
- `use perl_builtins::builtin_signatures::X;` -> `use perl_lexer::X;` (via api.rs) or `use perl_lexer::builtins::builtin_signatures::X;` (module-path)
- `use perl_builtins::builtin_signatures_phf::X;` -> `use perl_lexer::X;` (via api.rs) or `use perl_lexer::builtins::phf_lookup::X;`
- `use perl_builtins_phf::X;` -> same as above
- `use perl_tokenizer::TokenKind;` -> `use perl_token::TokenKind;` (direct) or `use perl_lexer::tokenizer::TokenKind;` (re-export)
- `use perl_tokenizer::token_stream::TokenStream;` -> `use perl_lexer::tokenizer::token_stream::TokenStream;` or `use perl_lexer::TokenStream;`
- `use perl_tokenizer::token_wrapper::PositionTracker;` -> `use perl_lexer::tokenizer::token_wrapper::PositionTracker;`
- `use perl_tokenizer::util::{code_slice, find_data_marker_byte_lexed};` -> `use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};`

**Facade API completeness test (NEW, Wave 1 pattern):**
- Add `crates/perl-lexer/tests/facade_api_completeness.rs` -- smoke-imports every item re-exported from `api.rs`, guards accidental API breakage. Required per Wave 1 learning.

### 8. Trivia tests follow the trivia modules to `perl-parser-core`

- **Decision:** `trivia_edge_cases.rs` (currently in perl-tokenizer/tests/) contains only trivia-specific assertions. It should migrate to `perl-parser-core/tests/` as `trivia_edge_cases.rs` with imports rewritten to `perl_parser_core::{trivia, trivia_parser}`.
- **Split test files:** `comprehensive_unit_tests.rs`, `extended_unit_tests.rs`, and `bridge_coverage_tests.rs` from perl-tokenizer contain BOTH AST-agnostic (TokenStream/PositionTracker/util) and trivia tests. For Wave C, migrate these whole files to `perl-lexer/tests/tokenizer_*.rs` with `use perl_parser_core::trivia::...;` imports where needed. (perl-lexer is NOT a dev-dep consumer of perl-parser-core -- create a cycle? No: these are integration tests in `perl-lexer/tests/` that naturally live outside the crate graph; dev-deps can include perl-parser-core. BUT this adds perl-parser-core as a dev-dep of perl-lexer, which is acceptable for tests but noisy.)
- **Alternative (cleaner):** Move the trivia-touching subset of those three test files to `perl-parser-core/tests/` (where trivia ends up), and keep only AST-agnostic portions in `perl-lexer/tests/tokenizer_*.rs`. Builder may choose whichever produces the cleanest test file split.
- **Pragmatic choice:** Migrate whole test files to `perl-lexer/tests/` with `perl-parser-core` as a dev-dep. This mirrors the existing coupling of these tests (they already reach across the tokenizer/trivia boundary). Accepting dev-dep noise over test fragmentation.
- **Add to `perl-lexer/Cargo.toml` dev-dependencies:** `perl-parser-core = { workspace = true }` (for the migrated trivia-reaching tests).

### 9. Consumer workspace count and publish allowlist

**Baseline (from `cargo metadata` on current master post-#4446):**
- Workspace members: **101**
- Publish allowlist entries: **98**

**Post-Wave C:**
- Workspace members: **97** (-4)
- Publish allowlist: **94** (-4)

**Note:** These numbers differ slightly from the plan-review summary ("100 -> 96"); that referred to a different count method. Use the `cargo metadata` numbers as ground truth in builder verification.

### 10. Backup file cleanup

- Delete `crates/perl-tokenizer/src/trivia_parser.rs.backup` as part of Phase 6 (satellite directory deletion is a superset of this, but call it out explicitly -- oppositional-planner found it).

---

## Alternatives Considered

### A1: Absorb `perl-token` too (ORIGINAL scope)

Rejected via ADR Amendment 4 (#4446). `perl-token` is consumed by the entire analysis stack (lexer, parser, semantic, LSP, DAP); folding it into `perl-lexer` would force every downstream consumer to depend on the full lexer just to get the `Token` / `TokenKind` types.

### A2: Keep `perl-keywords` published separately (oppositional A1)

Considered in plan-review. Rejected because keywords is a pure lookup table with no external deps; absorbing it is low risk and follows the collapse target. The architecture-reviewer confirmed ALIGNED -- keywords are consumed downstream of the lexer, so re-exporting them via `perl_lexer::*` is sound.

### A3: Move trivia modules into `perl-parser` instead of `perl-parser-core`

`perl-parser` depends on `perl-parser-core` and already re-exports `perl_parser_core::tokens::*`. Moving trivia logic to `perl-parser-core` (decision 3) is preferred because:
- `perl-parser-core` already re-exports them at `perl_parser_core::{trivia, trivia_parser}` -- consumers don't change
- `perl-parser-core` already depends on `perl-ast-v2`
- `perl-parser` is a larger consumer surface; better to keep the internal modules in parser-core

### A4: Inline all 4 satellites as flat files (no folders)

Rejected. Wave 1 / Wave A / Wave B all use folder modules; consistency matters for the ADR-0041 collapse pattern.

---

## Edge Cases and Risk Flags

### E1: Cyclic-dep risk with trivia relocation

`perl-parser-core` takes on the trivia modules. Its test file `trivia_extended_tests.rs` already uses `use perl_parser_core::trivia::Trivia;` -- no import change needed.

`perl-lexer/tests/` uses `perl-parser-core` as a dev-dep for the migrated trivia-touching tests (decision 8). This is NOT a cyclic dep (dev-deps don't count for lib graph) but it IS unusual. Verify with `cargo xtask layer-check` which checks lib-graph only.

### E2: `perl_keywords::is_lexer_keyword` used INSIDE perl-lexer (line 139 of lib.rs)

Currently: `use perl_keywords::is_lexer_keyword;`. After absorption: `use crate::keywords::is_lexer_keyword;`. The module must be declared `pub mod keywords;` in lib.rs BEFORE this `use` line or after (Rust allows forward module declarations, so location doesn't matter -- place all module declarations together).

### E3: PR #4446 just merged (2026-04-17)

Branch must base on `origin/master` with commit `6992efcb9` or newer. Base-from-master at the start of spec-planner (already done by git checkout above).

### E4: `builtin_signatures_phf` public path change

`perl-builtins/src/lib.rs:9` does `pub use perl_builtins_phf as builtin_signatures_phf;`. After absorption, this re-alias becomes: inside `perl-lexer/src/builtins/mod.rs`, we declare `pub mod phf_lookup;` (the absorbed PHF file) and **also** add `pub use phf_lookup as builtin_signatures_phf;` to preserve the public path for consumers that reach through `perl_builtins::builtin_signatures_phf::*`. Then api.rs can re-export both names.

### E5: Edition 2024 inherited

Workspace pins `edition = "2024"`. `perl-lexer/Cargo.toml` uses `edition.workspace = true`. Verified (Wave 1 gotcha: builder forgot this).

### E6: Windows MAX_PATH at deep worktrees

This is a main-checkout spec, no worktree. Builder should keep operations in the main checkout too if MAX_PATH would bite. (MEMORY gotcha from Wave 1.)

### E7: `unreachable_pub` lint in perl-parser-core

`perl-parser-core/src/lib.rs:48` has `#![deny(unreachable_pub)]`. When the new trivia modules move into parser-core, they must have consistent `pub` visibility that doesn't trigger this lint -- keep their public items `pub` (reachable via `pub mod trivia`/`pub mod trivia_parser` declarations).

### E8: Facade test compilation catches API drift

`facade_api_completeness.rs` (decision 7) is the gate for "did we forget to re-export something." If a test import fails, add to api.rs.

### E9: `format_with_trivia` is a fn, not a type

Decision 4 api.rs re-exports list: check that `format_with_trivia` (in trivia_parser) is INCLUDED if it's part of the pre-collapse public surface; after trivia moves to parser-core, it stays a public fn on `perl_parser_core::trivia_parser::format_with_trivia` (already re-exported in parser-core lib.rs:158).

---

## References

- **ADR-0041:** `docs/adr/0041-microcrate-collapse.md`
- **Ledger:** `.spec/microcrate-collapse/ledger.md` (Wave 3 / Wave C row)
- **Amendment 4:** PR #4446 (perl-token stays published), merged at `6992efcb9`
- **Wave 1 pilot:** PR #4422 perl-module-* collapse
- **Wave A:** PR #4434 perl-workspace-* collapse
- **Wave B:** PR #4438 perl-symbol-* collapse (4 satellites -> new perl-symbol)
- **Wave E:** PR #4435 perl-diagnostics creation
- **Wave H:** PR #4433 perl-dap-* collapse

---

## Summary of Orchestrator Decisions (locked)

1. 4 satellites absorbed (NOT 5); perl-token stays published.
2. Target crate is existing `perl-lexer`; no rename.
3. Flat module layout inside `perl-lexer/src/`: `keywords/`, `builtins/`, `tokenizer/`.
4. PHF as `builtins/phf_lookup.rs` (single file), NOT a folder.
5. Existing `perl-lexer/src/token.rs` stays untouched.
6. Tokenizer AST-agnostic slice only: `token_stream`, `token_wrapper`, `util`.
7. Trivia modules (`trivia.rs`, `trivia_parser.rs`) move to `perl-parser-core/src/tokens/`.
8. `api.rs` with explicit re-exports (no wildcards).
9. 9 consumers need dep/import updates.
10. 14 test files migrate with Wave 1 prefix pattern; +1 facade_api_completeness.rs.
11. Workspace 101 -> 97 (-4); allowlist 98 -> 94 (-4).

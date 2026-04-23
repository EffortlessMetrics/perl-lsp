# Implementation Checklist: Wave C Microcrate Collapse (lexer satellites)

**Issue:** #4444
**Branch:** `impl/4444-perl-lexer-wave-c`
**Target:** Absorb 4 satellites (`perl-tokenizer`, `perl-keywords`, `perl-builtins`, `perl-builtins-phf`) into the existing published `perl-lexer` crate. Move trivia modules to `perl-parser-core`. Delete 4 old directories.
**Test counts:** 14 absorbed test files + 1 new `facade_api_completeness.rs` = 15 tests landing in `perl-lexer/tests/` (or `perl-parser-core/tests/` for trivia-specific).
**Crate touch:** `perl-lexer` (absorbing), `perl-parser-core` (gets trivia modules), 9 consumer crates, root `Cargo.toml`.

---

## Preconditions

- [ ] #4446 (perl-token ADR amendment) merged. Verify: `git log --oneline origin/master | grep -i "perl-token stays published"` shows commit `6992efcb9` or similar.
- [ ] Branch `impl/4444-perl-lexer-wave-c` is based on current `origin/master` post-#4446.
- [ ] No uncommitted changes: `git status`.
- [ ] **Pre-change baselines captured:** `cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; d=json.load(sys.stdin); print('Members:', len(d['workspace_members']))"` -- record the count (expect 101). Similarly record allowlist count from `[workspace.metadata.publish].allow` in root Cargo.toml (expect 98).

---

## Phase 1: Prepare `perl-lexer` for Absorption

### Step 1.1: Add `phf` external dependency to `perl-lexer/Cargo.toml`

**File:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/Cargo.toml`

**Action:** In `[dependencies]`, add between `memchr` and `tracing`:

```toml
phf = { version = "0.13.1", features = ["macros"] }
```

Also add `perl-parser-core = { workspace = true }` to `[dev-dependencies]` (only needed for migrated trivia-touching tests per context decision 8 -- the dev-dep is fine, no lib cycle).

**Remove** the line `perl-keywords = { workspace = true }` (currently line 26) because keywords becomes an internal module.

**Verify:** `cargo metadata --no-deps --manifest-path crates/perl-lexer/Cargo.toml 2>&1 | head -3` parses cleanly.

---

### Step 1.2: Update `perl-lexer/src/lib.rs` -- declare absorbed modules

**File:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/lib.rs`

**Action:**

1. Replace line 139 `use perl_keywords::is_lexer_keyword;` with `use crate::keywords::is_lexer_keyword;`.

2. In the module declaration block (currently lines 142-147), add the new modules. After the existing `pub mod token;`, add:

```rust
pub mod keywords;
pub mod builtins;
pub mod tokenizer;
pub mod api;
pub use api::*;
```

Final module declaration block should look like:

```rust
pub mod checkpoint;
pub mod error;
pub mod mode;
mod quote_handler;
pub mod token;
pub mod keywords;
pub mod builtins;
pub mod tokenizer;
pub mod api;
mod unicode;

pub use checkpoint::{CheckpointCache, Checkpointable, LexerCheckpoint};
pub use error::{LexerError, Result};
pub use mode::LexerMode;
pub use perl_position_tracking::Position;
pub use token::{StringPart, Token, TokenType};
pub use api::*;
```

**Verify:** `cargo check -p perl-lexer 2>&1 | head -20` -- will fail until modules exist (Phase 2). That's expected.

---

## Phase 2: Absorb Satellite Source into Modules

### Step 2.1: Create `src/keywords/mod.rs` from `perl-keywords/src/lib.rs`

**File:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/keywords/mod.rs`

**Action:** Copy complete content of `crates/perl-keywords/src/lib.rs` into the new file.

**Edits during copy:**
- Remove the crate-level `//!` module docstring block if redundant (perl-lexer lib.rs already describes the crate).
- Keep all `pub const` and `pub fn` items -- they will be re-exported via api.rs.
- No external crate imports (keywords has no workspace deps).

**Verify:** `cargo check -p perl-lexer 2>&1 | grep -E "error|keywords" | head -10`. Expect: resolves `use crate::keywords::is_lexer_keyword` on lib.rs:139.

---

### Step 2.2: Create `src/builtins/mod.rs` and `src/builtins/phf_lookup.rs`

**File A:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/builtins/phf_lookup.rs`

**Action:** Copy complete content of `crates/perl-builtins-phf/src/lib.rs` into this file (which becomes an internal file, NOT `mod.rs` -- it's a sibling of `mod.rs`).

**File B:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/builtins/mod.rs`

**Action:** Based on `crates/perl-builtins/src/lib.rs`:

```rust
//! Builtin function signatures and metadata for Perl.
//!
//! Provides [`BuiltinSignature`](builtin_signatures::BuiltinSignature) entries
//! covering Perl's built-in functions, including signature variants and
//! documentation strings. Used by the LSP completion, hover, and signature-help
//! providers to surface accurate information without an external Perl runtime.

pub mod builtin_signatures;
pub mod phf_lookup;

// Preserve legacy public path `perl_builtins::builtin_signatures_phf::*`
// (which is now accessible via `perl_lexer::builtins::builtin_signatures_phf::*`)
pub use phf_lookup as builtin_signatures_phf;
```

**File C:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/builtins/builtin_signatures.rs`

**Action:** Copy complete content of `crates/perl-builtins/src/builtin_signatures.rs` into this file. No internal path rewrites needed (the file uses only std::collections::HashMap).

**Verify:** `cargo check -p perl-lexer 2>&1 | grep -E "error|builtins" | head -15`.

---

### Step 2.3: Create `src/tokenizer/` (AST-agnostic slice only)

**Source files to absorb:** `token_stream.rs`, `token_wrapper.rs`, `util.rs` from `crates/perl-tokenizer/src/`.

**Files to create:**

1. `crates/perl-lexer/src/tokenizer/mod.rs`:
   ```rust
   //! Token stream and utilities bridging raw lexer output to parser consumption.
   //!
   //! This module consolidates the former `perl-tokenizer` crate's AST-agnostic
   //! slice: the buffered [`TokenStream`], position-tracking wrappers, and
   //! [`__DATA__`/`__END__`](util) marker utilities. Trivia preservation
   //! (comments/whitespace -> AST) lives in `perl-parser-core`, since it
   //! depends on `perl-ast-v2`.

   pub mod token_stream;
   pub mod token_wrapper;
   pub mod util;

   pub use perl_token::{Token, TokenKind};
   pub use token_stream::TokenStream;
   pub use token_wrapper::TokenWithPosition;
   ```

2. `crates/perl-lexer/src/tokenizer/token_stream.rs`: Copy content of `crates/perl-tokenizer/src/token_stream.rs`. Update any `use perl_lexer::` to `use crate::` (the file is now *inside* perl-lexer). Doc-test imports `use perl_tokenizer::{TokenKind, TokenStream};` in the module-level `//!` docstring must be rewritten to `use perl_lexer::tokenizer::{TokenKind, TokenStream};` (or use `use perl_lexer::{TokenKind, TokenStream};` via crate root if api.rs re-exports them).

3. `crates/perl-lexer/src/tokenizer/token_wrapper.rs`: Copy content of `crates/perl-tokenizer/src/token_wrapper.rs`. Rewrite `use perl_lexer::Token;` to `use crate::token::Token;` (ambiguity: `perl_lexer::Token` is lexer's own, NOT perl-token's -- verify which is in use). Look at the file: if it imports lexer's `Token` (from `perl_lexer::Token`, which is `perl_lexer::token::Token` per lib.rs:153), then `use crate::token::Token;` is correct. If it imports `perl_token::Token`, keep as-is (perl-token is a separate published crate).

4. `crates/perl-lexer/src/tokenizer/util.rs`: Copy content of `crates/perl-tokenizer/src/util.rs`. Rewrite `use perl_lexer::{PerlLexer, TokenType};` to `use crate::{PerlLexer, TokenType};`.

**Skip:** `crates/perl-tokenizer/src/trivia.rs`, `crates/perl-tokenizer/src/trivia_parser.rs`, `crates/perl-tokenizer/src/trivia_parser.rs.backup` -- these are handled in Phase 3 (moved to perl-parser-core) or deleted.

**Verify:** `cargo check -p perl-lexer 2>&1 | grep -E "error|tokenizer" | head -15`.

---

### Step 2.4: Create `src/api.rs` public facade

**File:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/src/api.rs`

**Content:**

```rust
//! Public API re-exports for `perl-lexer` post-collapse.
//!
//! This module defines the public surface contributed by the Wave C-absorbed
//! satellites (keywords, builtins, tokenizer). Lexer-native items
//! (Token/TokenType/StringPart, LexerMode, Checkpointable, Position, etc.)
//! continue to be re-exported from `lib.rs` directly.
//!
//! All re-exports here are explicit named items (no wildcards) so the
//! public contract is reviewable at a glance.

// keywords module
pub use crate::keywords::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

// builtins module
pub use crate::builtins::builtin_signatures::{BuiltinSignature, create_builtin_signatures};
pub use crate::builtins::phf_lookup::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};

// tokenizer module
pub use crate::tokenizer::token_stream::TokenStream;
pub use crate::tokenizer::token_wrapper::{PositionTracker, TokenWithPosition};
pub use crate::tokenizer::util::{code_slice, find_data_marker_byte, find_data_marker_byte_lexed};
```

**Notes:**
- `find_data_marker_byte` is deprecated in util.rs. Re-export anyway for backward compat; consumers who call it keep compiling.
- Do NOT re-export `perl_token::{Token, TokenKind}` via api.rs: those come from a sibling published crate, so consumers should import `perl_token::Token` directly or through `perl_lexer::tokenizer::{Token, TokenKind}` (re-exported in tokenizer/mod.rs).

**Verify:** `cargo check -p perl-lexer 2>&1 | tail -20`. Expect: clean check (all items resolve).

---

### Step 2.5: Full crate check

```bash
cd H:/Code/Rust/perl-lsp && cargo check -p perl-lexer 2>&1 | tail -10
```

**Expected:** Clean compile. If errors, resolve before Phase 3.

---

## Phase 3: Move Trivia Modules to `perl-parser-core`

### Step 3.1: Copy `trivia.rs` into parser-core

**Source:** `crates/perl-tokenizer/src/trivia.rs`
**Destination:** `crates/perl-parser-core/src/tokens/trivia.rs`

**Action:** Copy file content. The file already uses `use perl_ast_v2::{Node, NodeKind};` and `use perl_lexer::TokenType;` and `use perl_position_tracking::Range;` -- all three are valid in parser-core's dependency graph. No import rewrites needed.

---

### Step 3.2: Copy `trivia_parser.rs` into parser-core

**Source:** `crates/perl-tokenizer/src/trivia_parser.rs`
**Destination:** `crates/perl-parser-core/src/tokens/trivia_parser.rs`

**Action:** Copy file content. The file uses `use crate::trivia::{NodeWithTrivia, Trivia, TriviaToken};` -- since both files move to the same module folder (`src/tokens/`), this import becomes `use super::trivia::{NodeWithTrivia, Trivia, TriviaToken};` OR `use crate::tokens::trivia::{...};`. Use the `super` form for locality.

Also rewrite `use perl_ast_v2::{Node, NodeIdGenerator, NodeKind};` -- keep as-is (parser-core depends on perl-ast-v2).

Also rewrite `use perl_lexer::{PerlLexer, Token, TokenType};` -- keep as-is.

**Verify:** `cargo check -p perl-parser-core 2>&1 | grep -E "error|trivia" | head -15`.

---

### Step 3.3: Update `perl-parser-core/src/tokens/mod.rs`

**File:** `H:/Code/Rust/perl-lsp/crates/perl-parser-core/src/tokens/mod.rs`

**Current content:**

```rust
pub mod token_stream;
/// Token wrapper utilities for preserving original lexemes and trivia.
pub use perl_tokenizer::token_wrapper;
/// Trivia tokens (whitespace/comments) used for formatting and diagnostics.
pub use perl_tokenizer::trivia;
/// Trivia parser helpers for preserving formatting context.
pub use perl_tokenizer::trivia_parser;
```

**New content:**

```rust
//! Token stream and trivia utilities for parser workflows.

/// Token stream adapters used during the Parse stage for LSP workflows.
pub mod token_stream;
/// Token wrapper utilities for preserving original lexemes.
pub use perl_lexer::tokenizer::token_wrapper;
/// Trivia tokens (whitespace/comments/POD) used for formatting and diagnostics.
pub mod trivia;
/// Trivia-preserving parser helpers for formatting context.
pub mod trivia_parser;
```

**File:** `H:/Code/Rust/perl-lsp/crates/perl-parser-core/src/tokens/token_stream.rs`

**Action:** Change line 27 `pub use perl_tokenizer::token_stream::*;` -> `pub use perl_lexer::tokenizer::token_stream::*;`.

**Verify:** `cargo check -p perl-parser-core 2>&1 | grep -E "error|trivia|token" | head -15`.

---

### Step 3.4: Update `perl-parser-core/src/lib.rs`

**File:** `H:/Code/Rust/perl-lsp/crates/perl-parser-core/src/lib.rs`

**Changes:**
- Line 89 `pub use perl_builtins as builtins;` -> `pub use perl_lexer::builtins;`. This keeps the public path `perl_parser_core::builtins::{builtin_signatures, builtin_signatures_phf}` intact.
- Line 114 `pub use perl_tokenizer::util;` -> `pub use perl_lexer::tokenizer::util;`. This keeps `perl_parser_core::util::*` available.
- Other pub uses of `token_stream`, `token_wrapper`, `trivia`, `trivia_parser` at lines 144-151 stay because they go through `tokens::` (updated in step 3.3).
- Lines 154-158 remain (re-export through `tokens::`).

**Verify:** `cargo check -p perl-parser-core 2>&1 | tail -20`. Expect clean.

---

### Step 3.5: Migrate `trivia_extended_tests.rs`

**File:** `crates/perl-parser-core/tests/trivia_extended_tests.rs`

**Action:** Already uses `use perl_parser_core::trivia::Trivia;` -- no change required. Verify it still passes: `cargo test -p perl-parser-core --test trivia_extended_tests`.

---

### Step 3.6: Migrate `trivia_edge_cases.rs` from perl-tokenizer

**Source:** `crates/perl-tokenizer/tests/trivia_edge_cases.rs`
**Destination:** `crates/perl-parser-core/tests/trivia_edge_cases.rs`

**Edits:**
- `use perl_tokenizer::trivia::{Trivia, TriviaLexer};` -> `use perl_parser_core::trivia::{Trivia, TriviaLexer};`
- `use perl_tokenizer::trivia_parser::TriviaPreservingParser;` -> `use perl_parser_core::trivia_parser::TriviaPreservingParser;`

**Verify:** `cargo test -p perl-parser-core --test trivia_edge_cases 2>&1 | tail -10`.

---

## Phase 4: Migrate Test Files to `perl-lexer/tests/`

All tests land in `crates/perl-lexer/tests/`. Use prefix scheme to avoid collisions.

### Step 4.1: Tokenizer tests (5 files -- trivia_edge_cases goes to parser-core; see Phase 3.6)

Copy each source file to destination; rewrite imports per the pattern below.

**Source -> Destination:**
- `crates/perl-tokenizer/tests/comprehensive_unit_tests.rs` -> `crates/perl-lexer/tests/tokenizer_comprehensive_unit_tests.rs`
- `crates/perl-tokenizer/tests/extended_unit_tests.rs` -> `crates/perl-lexer/tests/tokenizer_extended_unit_tests.rs`
- `crates/perl-tokenizer/tests/edge_case_tests.rs` -> `crates/perl-lexer/tests/tokenizer_edge_case_tests.rs`
- `crates/perl-tokenizer/tests/tokenizer_bdd_consolidated.rs` -> `crates/perl-lexer/tests/tokenizer_bdd_consolidated.rs` (already has prefix)
- `crates/perl-tokenizer/tests/bridge_coverage_tests.rs` -> `crates/perl-lexer/tests/tokenizer_bridge_coverage_tests.rs`

**Import rewrites for each file:**
- `use perl_tokenizer::TokenKind;` -> `use perl_token::TokenKind;` (direct; perl-token is still published)
- `use perl_tokenizer::Token;` -> `use perl_token::Token;`
- `use perl_tokenizer::token_stream::TokenStream;` -> `use perl_lexer::tokenizer::token_stream::TokenStream;` or `use perl_lexer::TokenStream;` (via api.rs)
- `use perl_tokenizer::token_wrapper::PositionTracker;` -> `use perl_lexer::tokenizer::token_wrapper::PositionTracker;` or `use perl_lexer::PositionTracker;` (via api.rs)
- `use perl_tokenizer::trivia::{Trivia, TriviaLexer, TriviaToken};` -> `use perl_parser_core::trivia::{Trivia, TriviaLexer, TriviaToken};`
- `use perl_tokenizer::trivia_parser::{TriviaParserContext, TriviaPreservingParser};` -> `use perl_parser_core::trivia_parser::{TriviaParserContext, TriviaPreservingParser};`
- `use perl_tokenizer::trivia_parser::format_with_trivia;` -> `use perl_parser_core::trivia_parser::format_with_trivia;`
- `use perl_tokenizer::util::{code_slice, find_data_marker_byte_lexed};` -> `use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};` or `use perl_lexer::{code_slice, find_data_marker_byte_lexed};` (via api.rs)
- `perl_tokenizer::trivia_parser::format_with_trivia` (full-path inside tests at lines 810, 1049, 1881 of comprehensive_unit_tests.rs) -> `perl_parser_core::trivia_parser::format_with_trivia`

**Verify each:** `cargo test -p perl-lexer --test tokenizer_comprehensive_unit_tests 2>&1 | tail -5` (repeat for each of the 5 files).

---

### Step 4.2: Keywords tests (4 files)

**Source -> Destination:**
- `crates/perl-keywords/tests/comprehensive_unit_tests.rs` -> `crates/perl-lexer/tests/keywords_comprehensive_unit_tests.rs`
- `crates/perl-keywords/tests/keyword_classification.rs` -> `crates/perl-lexer/tests/keywords_classification.rs`
- `crates/perl-keywords/tests/keyword_edge_cases.rs` -> `crates/perl-lexer/tests/keywords_edge_cases.rs`
- `crates/perl-keywords/tests/bdd_keyword_qw.rs` -> `crates/perl-lexer/tests/keywords_bdd_qw.rs`

**Import rewrites:**
- `use perl_keywords::{ ... };` -> `use perl_lexer::{ ... };` (api.rs re-exports all 7 consts + 7 fns)

**Verify:** `cargo test -p perl-lexer --test keywords_comprehensive_unit_tests 2>&1 | tail -5` (and similar for each).

---

### Step 4.3: Builtins tests (3 files)

**Source -> Destination:**
- `crates/perl-builtins/tests/comprehensive_unit_tests.rs` -> `crates/perl-lexer/tests/builtins_comprehensive_unit_tests.rs`
- `crates/perl-builtins/tests/extended_unit_tests.rs` -> `crates/perl-lexer/tests/builtins_extended_unit_tests.rs`
- `crates/perl-builtins/tests/bdd_scenarios.rs` -> `crates/perl-lexer/tests/builtins_bdd_scenarios.rs`

**Import rewrites:**
- `use perl_builtins::builtin_signatures::create_builtin_signatures;` -> `use perl_lexer::create_builtin_signatures;` (api.rs) or `use perl_lexer::builtins::builtin_signatures::create_builtin_signatures;` (module-path).
- `use perl_builtins::builtin_signatures_phf::{BUILTIN_FULL_SIGS, get_param_names, is_builtin};` -> `use perl_lexer::{BUILTIN_FULL_SIGS, get_param_names, is_builtin};` (api.rs) or `use perl_lexer::builtins::phf_lookup::{BUILTIN_FULL_SIGS, get_param_names, is_builtin};`.
- `use perl_builtins::builtin_signatures::BuiltinSignature;` -> `use perl_lexer::BuiltinSignature;`.

**Verify:** `cargo test -p perl-lexer --test builtins_comprehensive_unit_tests 2>&1 | tail -5`.

---

### Step 4.4: Builtins-PHF tests (1 file)

**Source -> Destination:**
- `crates/perl-builtins-phf/tests/comprehensive_unit_tests.rs` -> `crates/perl-lexer/tests/builtins_phf_comprehensive_unit_tests.rs`

**Import rewrites:**
- `use perl_builtins_phf::{ ... };` -> `use perl_lexer::builtins::phf_lookup::{ ... };` or `use perl_lexer::{ ... };`.

**Verify:** `cargo test -p perl-lexer --test builtins_phf_comprehensive_unit_tests 2>&1 | tail -5`.

---

### Step 4.5: Create `facade_api_completeness.rs` (NEW, Wave 1 pattern)

**File:** `H:/Code/Rust/perl-lsp/crates/perl-lexer/tests/facade_api_completeness.rs`

**Content:**

```rust
//! Guards the public API surface contributed by Wave C absorption. If an item
//! listed here becomes inaccessible at the documented path, this test fails --
//! catching accidental API breakage during future refactoring.

use perl_lexer::{
    // keywords
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
    // builtins
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, BuiltinSignature, builtin_count,
    create_builtin_signatures, get_param_names, is_builtin,
    // tokenizer
    PositionTracker, TokenStream, TokenWithPosition, code_slice, find_data_marker_byte,
    find_data_marker_byte_lexed,
};

#[test]
fn keywords_accessible_at_crate_root() {
    assert!(!KEYWORDS.is_empty());
    assert!(is_keyword("my"));
    assert!(is_lexer_keyword("sub"));
    let _ = is_lsp_completion_keyword("use");
    let _ = is_dap_completion_keyword("step");
    let _ = is_lsp_runtime_completion_keyword("print");
    let _ = is_rename_keyword("my");
    let _ = is_parser_lsp_keyword("package");
    let _ = LSP_COMPLETION_KEYWORDS;
    let _ = DAP_COMPLETION_KEYWORDS;
    let _ = LSP_RUNTIME_COMPLETION_KEYWORDS;
    let _ = RENAME_KEYWORDS;
    let _ = PARSER_LSP_KEYWORDS;
    let _ = LEXER_KEYWORDS;
}

#[test]
fn builtins_accessible_at_crate_root() {
    assert!(is_builtin("print"));
    assert!(builtin_count() > 0);
    let _: &[&str] = get_param_names("substr");
    let _ = &BUILTIN_SIGS;
    let _ = &BUILTIN_FULL_SIGS;
    let sigs = create_builtin_signatures();
    let _: Option<&BuiltinSignature> = sigs.get("print");
}

#[test]
fn tokenizer_accessible_at_crate_root() {
    let mut stream = TokenStream::new("my $x = 1;");
    let _ = stream.peek();
    let _ = code_slice("print 1;\n__DATA__\nstuff");
    let _ = find_data_marker_byte_lexed("print 1;\n__DATA__\nstuff");
    let _: Option<usize> = find_data_marker_byte("print 1;\n");
    // Type-only smoke-checks
    let _: Option<TokenWithPosition> = None;
    let _: Option<PositionTracker<'_>> = None;
}
```

**Note on `PositionTracker`:** If the real type takes a lifetime parameter, adjust the test (`PositionTracker<'_>`). Compilation is the primary test; runtime execution is secondary.

**Verify:** `cargo test -p perl-lexer --test facade_api_completeness 2>&1 | tail -10`.

---

## Phase 5: Update Consumer Crates (9 consumers)

Each consumer needs a Cargo.toml update and source file rewrites. Order doesn't matter; tackle each in turn.

### Step 5.1: `perl-parser-core` (already partially done in Phase 3 -- finish it)

**Cargo.toml:** `crates/perl-parser-core/Cargo.toml`

**Changes:**
- Remove line 33 `perl-builtins = { workspace = true }` (now pulled in via `perl-lexer`).
- Remove line 37 `perl-tokenizer = { workspace = true }` (absorbed).
- `perl-lexer = { workspace = true }` already present (line 26) -- keep.

**Source files:** already edited in Phase 3.4.

**Verify:** `cargo build -p perl-parser-core 2>&1 | tail -10`.

---

### Step 5.2: `perl-dap`

**Cargo.toml:** `crates/perl-dap/Cargo.toml`

**Changes:** Line 61 `perl-keywords = { workspace = true }` -> remove. Add `perl-lexer = { workspace = true }` if not already present (check).

**Source:** `crates/perl-dap/src/debug_adapter/mod.rs`
- Line 43: `use perl_keywords::DAP_COMPLETION_KEYWORDS;` -> `use perl_lexer::DAP_COMPLETION_KEYWORDS;`

**Verify:** `cargo build -p perl-dap 2>&1 | tail -10`.

---

### Step 5.3: `perl-lsp`

**Cargo.toml:** `crates/perl-lsp/Cargo.toml`

**Changes:** Line 88 `perl-keywords = { workspace = true }` -> remove. Ensure `perl-lexer` (or a transitive crate re-exporting the needed items) is in deps.

**Source:** `crates/perl-lsp/src/runtime/language/completion.rs`
- Line 21: `use perl_keywords::LSP_RUNTIME_COMPLETION_KEYWORDS;` -> `use perl_lexer::LSP_RUNTIME_COMPLETION_KEYWORDS;`

**Verify:** `RUST_TEST_THREADS=2 cargo build -p perl-lsp-rs 2>&1 | tail -10`.

---

### Step 5.4: `perl-lsp-code-actions`

**Cargo.toml:** `crates/perl-lsp-code-actions/Cargo.toml`

**Changes:** Line 20 `perl-builtins = { workspace = true }` -> remove. Ensure `perl-lexer` is present in deps.

**Source:** `crates/perl-lsp-code-actions/src/enhanced/import_management.rs`
- Line 92: `use perl_builtins::builtin_signatures_phf::is_builtin;` -> `use perl_lexer::is_builtin;`

**Verify:** `cargo build -p perl-lsp-code-actions 2>&1 | tail -10`.

---

### Step 5.5: `perl-lsp-completion`

**Cargo.toml:** `crates/perl-lsp-completion/Cargo.toml`

**Changes:** Line 25 `perl-keywords = { workspace = true }` -> remove. Ensure `perl-lexer` is present.

**Source:** `crates/perl-lsp-completion/src/completion/keywords.rs`
- Line 11: `use perl_keywords::LSP_COMPLETION_KEYWORDS;` -> `use perl_lexer::LSP_COMPLETION_KEYWORDS;`

**Verify:** `cargo build -p perl-lsp-completion 2>&1 | tail -10`.

---

### Step 5.6: `perl-lsp-inlay-hints`

**Cargo.toml:** `crates/perl-lsp-inlay-hints/Cargo.toml`

**Changes:** Line 23 `perl-builtins = { workspace = true }` -> remove. Ensure `perl-lexer` is present.

**Source:** `crates/perl-lsp-inlay-hints/src/inlay_hints.rs`
- Line 21: `use perl_builtins::builtin_signatures::create_builtin_signatures;` -> `use perl_lexer::create_builtin_signatures;`

**Verify:** `cargo build -p perl-lsp-inlay-hints 2>&1 | tail -10`.

---

### Step 5.7: `perl-lsp-rename`

**Cargo.toml:** `crates/perl-lsp-rename/Cargo.toml`

**Changes:** Line 22 `perl-keywords = { workspace = true }` -> remove. Ensure `perl-lexer` is present.

**Source:** `crates/perl-lsp-rename/src/rename/validate.rs`
- Line 5: `use perl_keywords::is_rename_keyword;` -> `use perl_lexer::is_rename_keyword;`

**Verify:** `cargo build -p perl-lsp-rename 2>&1 | tail -10`.

---

### Step 5.8: `perl-parser`

**Cargo.toml:** `crates/perl-parser/Cargo.toml`

**Changes:** Line 35 `perl-keywords = { workspace = true }` -> remove. Ensure `perl-lexer` is present (via `perl-parser-core` transitive, or add direct).

**Source files:**
- `crates/perl-parser/src/ide/lsp_compat/rename.rs` line 59: `use perl_keywords::is_parser_lsp_keyword;` -> `use perl_lexer::is_parser_lsp_keyword;`
- `crates/perl-parser/src/ide/lsp_compat/completion.rs` line 64: `use perl_keywords::PARSER_LSP_KEYWORDS;` -> `use perl_lexer::PARSER_LSP_KEYWORDS;`

**Verify:** `cargo build -p perl-parser 2>&1 | tail -10`.

---

### Step 5.9: `perl-lexer` itself (internal use)

Already done in Phase 1.2 (lib.rs line 139). Re-verify the import works: `cargo check -p perl-lexer 2>&1 | tail -5`.

---

## Phase 6: Root `Cargo.toml` Updates

### Step 6.1: Update `[workspace.members]`

**File:** `H:/Code/Rust/perl-lsp/Cargo.toml`

**Remove** (4 entries):
- `"crates/perl-token",` -- NO! perl-token stays published. Keep.

Remove these four lines only:
- `"crates/perl-builtins",` (line 10)
- `"crates/perl-builtins-phf",` (line 11)
- `"crates/perl-tokenizer",` (line 16)
- `"crates/perl-keywords",` (line 85)

**Verify:**
```bash
cd H:/Code/Rust/perl-lsp && python3 -c "
import re
with open('Cargo.toml') as f: text = f.read()
members = re.search(r'members\s*=\s*\[(.*?)\]', text, re.DOTALL).group(1)
mems = re.findall(r'\"([^\"]+)\"', members)
print('Members:', len(mems))
for target in ['perl-tokenizer','perl-keywords','perl-builtins','perl-builtins-phf']:
    assert f'crates/{target}' not in mems, f'still in members: {target}'
    print(f'removed: {target}')
print('perl-token present:', 'crates/perl-token' in mems)
"
```

**Expected:** Members count = 97, all 4 removed, perl-token present.

---

### Step 6.2: Update `[workspace.dependencies]`

Remove these four entries:
- Line 258: `perl-builtins = { path = "crates/perl-builtins", version = "0.12.4" }`
- Line 259: `perl-builtins-phf = { path = "crates/perl-builtins-phf", version = "0.12.4" }`
- Line 267: `perl-tokenizer = { path = "crates/perl-tokenizer", version = "0.12.4" }`
- Line 272: `perl-keywords = { path = "crates/perl-keywords", version = "0.12.4" }`

**Keep:** `perl-token = { path = "crates/perl-token", version = "0.12.4" }` (line 255).

**No new entry needed** -- `perl-lexer` already exists at line 262.

**Verify:** `grep -nE '^perl-(tokenizer|keywords|builtins|builtins-phf)' Cargo.toml` should return zero hits.

---

### Step 6.3: Update `[workspace.metadata.publish].allow`

Remove these four lines from the allowlist:
- `"perl-builtins",`
- `"perl-builtins-phf",`
- `"perl-keywords",`
- `"perl-tokenizer",`

**Keep:** `"perl-token"`, `"perl-lexer"`.

**No new entry needed** -- `perl-lexer` is already in the allowlist (line 138).

**Verify:**
```bash
cd H:/Code/Rust/perl-lsp && python3 -c "
import re
with open('Cargo.toml') as f: text = f.read()
m = re.search(r'\[workspace\.metadata\.publish\]\s*\nallow\s*=\s*\[(.*?)\]', text, re.DOTALL)
allow = re.findall(r'\"([^\"]+)\"', m.group(1))
print('Allowlist count:', len(allow))
for target in ['perl-tokenizer','perl-keywords','perl-builtins','perl-builtins-phf']:
    assert target not in allow, f'still in allowlist: {target}'
    print(f'removed: {target}')
print('perl-token present:', 'perl-token' in allow)
print('perl-lexer present:', 'perl-lexer' in allow)
"
```

**Expected:** Allowlist count = 94.

---

### Step 6.4: Verify workspace parses

```bash
cd H:/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 2>&1 | head -3
```

**Expected:** JSON output with no parse errors.

---

## Phase 7: Delete Old Crate Directories

### Step 7.1: Delete 4 satellite directories

```bash
rm -rf H:/Code/Rust/perl-lsp/crates/perl-tokenizer \
       H:/Code/Rust/perl-lsp/crates/perl-keywords \
       H:/Code/Rust/perl-lsp/crates/perl-builtins \
       H:/Code/Rust/perl-lsp/crates/perl-builtins-phf
```

**Note:** This also removes the orphan `crates/perl-tokenizer/src/trivia_parser.rs.backup` file.

**Verify:**

```bash
ls -d H:/Code/Rust/perl-lsp/crates/perl-tokenizer \
      H:/Code/Rust/perl-lsp/crates/perl-keywords \
      H:/Code/Rust/perl-lsp/crates/perl-builtins \
      H:/Code/Rust/perl-lsp/crates/perl-builtins-phf 2>&1
```

Expected: all four `No such file or directory`.

---

## Phase 8: Hygiene / Hardcoded Strings

### Step 8.1: Scan for stale crate name references

```bash
cd H:/Code/Rust/perl-lsp && grep -rn 'perl_tokenizer\|perl_keywords\|perl_builtins' crates/ --include='*.rs' 2>&1 | head -20
cd H:/Code/Rust/perl-lsp && grep -rn 'perl-tokenizer\|perl-keywords\|perl-builtins\|perl-builtins-phf' crates/ --include='*.toml' 2>&1 | head -20
```

**Expected:** Zero hits after Phases 5-7. If any remain, fix them (check especially `perl-ci-hygiene` — Wave A and B had hygiene-string hits).

---

### Step 8.2: Scan docs and scripts (lower priority)

```bash
cd H:/Code/Rust/perl-lsp && grep -rn 'perl-tokenizer\|perl-keywords\|perl-builtins\|perl-builtins-phf' docs/ scripts/ .github/ 2>&1 | head -20
```

**Expected:** Any remaining references are in ADR / ledger / changelog files (historical) -- leave those untouched. Only fix live references (script inputs, CI config).

---

## Phase 9: Final Verification

### Step 9.1: No old crate names in source

```bash
cd H:/Code/Rust/perl-lsp && grep -rn 'perl_tokenizer\|perl_keywords\|perl_builtins\|perl_builtins_phf' crates/ --include='*.rs' 2>&1 | grep -v '.spec/'
```

**Expected:** Zero hits.

```bash
cd H:/Code/Rust/perl-lsp && grep -rn 'perl-tokenizer\|perl-keywords\|perl-builtins\|perl-builtins-phf' crates/ --include='*.toml' 2>&1
```

**Expected:** Zero hits (only the removed ones).

---

### Step 9.2: Workspace member count

```bash
cd H:/Code/Rust/perl-lsp && cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print('Members:', len(d['workspace_members']))"
```

**Expected:** 97 (down from 101, delta -4).

---

### Step 9.3: Publish allowlist count

```bash
cd H:/Code/Rust/perl-lsp && cargo xtask publish-closure 2>&1 | tail -15
```

**Expected:**
- `perl-lexer` present; no `perl-tokenizer`, `perl-keywords`, `perl-builtins`, or `perl-builtins-phf`.
- Allowlist count = 94 (down from 98).
- `perl-token` still present.

---

### Step 9.4: Full `cargo test -p perl-lexer`

```bash
cd H:/Code/Rust/perl-lsp && cargo test -p perl-lexer 2>&1 | tail -15
```

**Expected:** All test binaries pass. Count includes:
- 19 pre-existing `perl-lexer/tests/` files
- 14 migrated files (5 tokenizer + 4 keywords + 3 builtins + 1 builtins-phf + skip trivia_edge_cases which went to parser-core)
- 1 new `facade_api_completeness.rs`
- Total: ~34 test binaries in `perl-lexer/tests/`.

---

### Step 9.5: Full workspace tests

```bash
cd H:/Code/Rust/perl-lsp && cargo test --workspace --lib 2>&1 | tail -30
```

**Expected:** All pass (excluding `.ci/blockers.yaml` entries).

---

### Step 9.6: LSP test threading

```bash
cd H:/Code/Rust/perl-lsp && RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2 2>&1 | tail -20
```

**Expected:** perl-lsp-rs tests pass with threading constraint.

---

### Step 9.7: Clippy and format

```bash
cd H:/Code/Rust/perl-lsp && cargo clippy --workspace --lib 2>&1 | tail -20
cd H:/Code/Rust/perl-lsp && cargo xtask fmt 2>&1 | tail -5
```

**Expected:** No new warnings; no formatting diff.

---

### Step 9.8: Layer check (architectural contract)

```bash
cd H:/Code/Rust/perl-lsp && cargo xtask layer-check 2>&1 | tail -10
```

**Expected:**
- `perl-lexer` has no downstream deps on parser/semantic-analyzer/LSP crates.
- `perl-lexer`'s direct lib-deps: `perl-position-tracking`, `unicode-ident`, `memchr`, `tracing`, `thiserror`, `phf`.
- `perl-lexer`'s dev-deps (noisy but allowed): `criterion`, `proptest`, `perl-parser-core`.

---

### Step 9.9: All 9 consumers build individually

```bash
cd H:/Code/Rust/perl-lsp && \
  cargo build -p perl-dap && \
  cargo build -p perl-lsp-code-actions && \
  cargo build -p perl-lsp-completion && \
  cargo build -p perl-lsp-inlay-hints && \
  cargo build -p perl-lsp-rename && \
  cargo build -p perl-lsp-rs && \
  cargo build -p perl-parser && \
  cargo build -p perl-parser-core && \
  cargo build -p perl-lexer && \
  echo "All 9 consumer/absorbed crates built successfully"
```

---

## Compilation Checkpoints

| After Phase | Expected |
|-------------|----------|
| 1 | `cargo check -p perl-lexer` may fail (modules empty) -- OK |
| 2 | `cargo check -p perl-lexer` MUST succeed |
| 3 | `cargo check -p perl-parser-core` MUST succeed |
| 4 | `cargo test -p perl-lexer` ~34 test binaries pass |
| 5 | Each consumer builds individually |
| 6 | `cargo metadata --no-deps` MUST succeed |
| 7 | No old satellite directories |
| 8 | No stale crate name strings in `crates/` |
| 9 | Full verification pass -- all green |

---

## Notes for Builder

1. **Pre-change baselines** (Phase 0): Run `cargo metadata --no-deps` and note members count (101) and allowlist count (98). Expected delta: members -4, allowlist -4.

2. **Edition 2024:** `edition.workspace = true` in the existing `perl-lexer/Cargo.toml`. Confirmed at workspace root. No change needed (Wave 1 gotcha — safe here since perl-lexer already uses workspace-inherited edition).

3. **`[lib] doctest = false`:** `perl-lexer/Cargo.toml` currently does NOT have an explicit `[lib] doctest = false` section. Its doctests currently run. When absorbing crates that had `[lib] doctest = false` (e.g., perl-tokenizer line 24 via `#![cfg_attr(test, allow(...))]`), you may need to adjust. Recommendation: leave perl-lexer's current doctest behavior as-is; the absorbed modules' doctests will run. If doctests fail on absorbed code (e.g., `use perl_tokenizer::` imports in doc examples), rewrite them to `use perl_lexer::` or disable via `/// ```ignore`.

4. **`PerlLexer` is pub in lib.rs:** The existing `PerlLexer` type is re-exported. Internal `use crate::PerlLexer;` (in the moved util.rs) works.

5. **Consumer dev-dep gate:** After Phase 5, run `cargo build --workspace --lib` (lib-only, skips dev-deps) to verify no lib-cycle. Then `cargo build --workspace --tests` for the full gate.

6. **Commit strategy:** One commit per phase for clarity; final PR squashes to single commit. Conventional commit: `refactor(lexer): collapse lexer satellites -> perl-lexer (Wave C) (#4444)`.

7. **PR title suffix:** Must end with `(#4444)` for validate-title CI (MEMORY: `feedback_validate_title_issue_ref.md`).

8. **Branch base:** Branch already created from current origin/master (includes #4446). Rebase only if further Wave commits land.

9. **MAX_PATH on Windows (MEMORY):** Spec-planner operates in main checkout, not worktree, to avoid `MAX_PATH` exhaustion. Builder should do the same.

10. **Trivia move is the risky part.** Verify after Phase 3 that:
    - `cargo test -p perl-parser-core --test trivia_extended_tests` passes (existing test, zero import changes).
    - `cargo test -p perl-parser-core --test trivia_edge_cases` passes (migrated from tokenizer).
    - `grep -rn 'perl_tokenizer::trivia' crates/` returns zero hits.

---

## Change Order Summary

1. Add phf dep, remove perl-keywords dep; update lib.rs module declarations (Phase 1).
2. Create keywords/, builtins/, tokenizer/ modules; create api.rs (Phase 2).
3. Move trivia.rs + trivia_parser.rs to perl-parser-core/src/tokens/; update parser-core re-exports (Phase 3).
4. Migrate 14 test files with prefix scheme + 1 new facade test (Phase 4).
5. Update 9 consumer Cargo.tomls and source files (Phase 5).
6. Edit root Cargo.toml: remove 4 members, 4 deps, 4 allowlist entries (Phase 6).
7. Delete 4 satellite directories (Phase 7).
8. Scan for stale refs; clean up (Phase 8).
9. Full verification: metadata, publish-closure, test, clippy, fmt, layer-check (Phase 9).

**Estimated effort:** 4-6 hours. Bulk mechanical; trivia relocation is the one architectural operation.

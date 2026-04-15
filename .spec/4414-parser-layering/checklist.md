# Implementation Checklist: Remove LSP Provider Re-exports from perl-parser

Restore parser as pure leaf crate by removing LSP provider dependencies and re-exports. This is PR #0 of the microcrate collapse roadmap (#4410).

## Change Order

All steps must be executed sequentially. Each step includes a verify command that must succeed before proceeding to the next.

### Step 1: Remove LSP crate dependencies from Cargo.toml

**File**: `crates/perl-parser/Cargo.toml`

**Change**: Delete lines 35-42 (the 8 LSP crate dependencies)
- `perl-lsp-code-actions = { workspace = true }`
- `perl-lsp-completion = { workspace = true }`
- `perl-lsp-diagnostics = { workspace = true }`
- `perl-lsp-inlay-hints = { workspace = true }`
- `perl-lsp-navigation = { workspace = true }`
- `perl-lsp-rename = { workspace = true }`
- `perl-lsp-semantic-tokens = { workspace = true }`
- `perl-lsp-tooling = { workspace = true }`

**Verify**: 
```bash
cargo build -p perl-parser --dry-run 2>&1 | grep -E "error|warning" | head -20
```
Expected: Build to fail (unresolved re-exports).

---

### Step 2: Remove LSP provider re-export blocks from lib.rs

**File**: `crates/perl-parser/src/lib.rs`

**Change**: Delete lines 437-496 (LSP provider re-export modules)
- Includes: `code_actions`, `completion`, `diagnostics`, `document_links`, `implementation_provider`, `inlay_hints`, `inlay_hints_provider`, `references`, `rename`, `semantic_tokens`, `semantic_tokens_provider`, `type_definition`, `type_hierarchy`, `workspace_symbols`
- Starting comment: `// Re-exports from extracted microcrates`
- Ends at closing `}` of `workspace_symbols` module (line 496)

**Critical**: Do NOT delete lines 498-513. These are legitimate re-exports that must be preserved:
- Lines 498-505: `refactor::import_optimizer`, `refactor::modernize`, `refactor::modernize_refactored`, `refactor::refactoring`
- Lines 507-513: `tokens::token_stream`, `tokens::token_wrapper`, `tokens::trivia`, `tokens::trivia_parser`

**Verify**:
```bash
grep -n "pub mod code_actions" /h/Code/Rust/perl-lsp/crates/perl-parser/src/lib.rs
```
Expected: (empty)

---

### Step 3: Remove tooling re-exports and declaration from lib.rs

**File**: `crates/perl-parser/src/lib.rs`

**Changes**:
1. Delete lines 514-519 (tooling re-exports):
   - `pub use tooling::performance;`
   - `pub use tooling::perl_critic;`
   - `pub use tooling::perltidy;`

2. Find and delete the `mod tooling;` declaration (currently at line 412)

**Verify**:
```bash
grep -n "pub use tooling::" /h/Code/Rust/perl-lsp/crates/perl-parser/src/lib.rs
grep -n "^pub mod tooling" /h/Code/Rust/perl-lsp/crates/perl-parser/src/lib.rs
```
Expected: (both empty)

---

### Step 4: Delete the tooling.rs file entirely

**File**: `crates/perl-parser/src/tooling.rs`

**Change**: Delete entire file

**Verify**:
```bash
test -f /h/Code/Rust/perl-lsp/crates/perl-parser/src/tooling.rs && echo "FAIL: file still exists" || echo "OK"
```
Expected: OK

---

### Step 5: Update live code consumer in perl-lsp/src/lib.rs

**File**: `crates/perl-lsp/src/lib.rs`

**Change**: Line 428 (inside `pub mod prelude`)
- Find: `pub use perl_parser::perl_critic::*;`
- Replace with: `pub use perl_lsp_tooling::perl_critic::*;`

**Verify**:
```bash
grep -n "pub use perl_lsp_tooling::perl_critic" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/lib.rs
```
Expected: `428:    pub use perl_lsp_tooling::perl_critic::*;`

---

### Step 6: Update live code consumer in diagnostics/pull.rs

**File**: `crates/perl-lsp/src/features/diagnostics/pull.rs`

**Change**: Line 335 (inside `run_perl_critic` function)
- Find: `use perl_parser::perl_critic::BuiltInAnalyzer;`
- Replace with: `use perl_lsp_tooling::perl_critic::BuiltInAnalyzer;`

**Verify**:
```bash
grep -n "use perl_lsp_tooling::perl_critic" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/features/diagnostics/pull.rs
```
Expected: `335:        use perl_lsp_tooling::perl_critic::BuiltInAnalyzer;`

---

### Step 7: Update test file consumer

**File**: `crates/perl-parser/tests/ast_snapshot_tests.rs`

**Change**: Line 13
- Find: `use perl_parser::{Parser, semantic_tokens};`
- Replace with:
  ```rust
  use perl_parser::Parser;
  use perl_lsp_semantic_tokens as semantic_tokens;
  ```

**Rationale**: `semantic_tokens` module is no longer re-exported from `perl_parser`. Import directly from `perl_lsp_semantic_tokens` and alias it to avoid changing call sites (lines 265, 272, 280 all use `semantic_tokens::legend()`).

**Verify**:
```bash
grep -n "use perl_lsp_semantic_tokens as semantic_tokens" /h/Code/Rust/perl-lsp/crates/perl-parser/tests/ast_snapshot_tests.rs
```
Expected: `13:use perl_lsp_semantic_tokens as semantic_tokens;`

---

### Step 8: Update documentation example in LSP_IMPLEMENTATION_GUIDE.md (3 locations)

**File**: `docs/reference/LSP_IMPLEMENTATION_GUIDE.md`

**Changes**:
1. Line 156: `use perl_parser::completion::CompletionProvider;` → `use perl_lsp_completion::CompletionProvider;`
2. Line 1056: `use perl_parser::semantic_tokens::encode_semantic_tokens;` → `use perl_lsp_semantic_tokens::encode_semantic_tokens;`
3. Line 1070: `use perl_parser::semantic_tokens_provider::{SemanticTokenType, SemanticTokenModifier};` → `use perl_lsp_semantic_tokens::{SemanticTokenType, SemanticTokenModifier};`

**Verify**:
```bash
grep -n "use perl_lsp_completion::CompletionProvider\|use perl_lsp_semantic_tokens::encode_semantic_tokens\|use perl_lsp_semantic_tokens::{SemanticTokenType" /h/Code/Rust/perl-lsp/docs/reference/LSP_IMPLEMENTATION_GUIDE.md
```
Expected: 3 matches

---

### Step 9: Update documentation examples in LSP_PROVIDERS_REFERENCE.md (3 locations)

**File**: `docs/reference/LSP_PROVIDERS_REFERENCE.md`

**Changes**:
1. Line 43: `use perl_parser::document_links::compute_links;` → `use perl_lsp_navigation::compute_links;`
2. Line 106: `use perl_parser::document_links::compute_links;` → `use perl_lsp_navigation::compute_links;`
3. Line 1243: `use perl_parser::implementation_provider::ImplementationProvider;` → `use perl_lsp_navigation::ImplementationProvider;`

**Verify**:
```bash
grep -n "use perl_lsp_navigation::compute_links\|use perl_lsp_navigation::ImplementationProvider" /h/Code/Rust/perl-lsp/docs/reference/LSP_PROVIDERS_REFERENCE.md
```
Expected: 3 matches (2 compute_links, 1 ImplementationProvider)

---

### Step 10: Update documentation example in IMPORT_OPTIMIZER_GUIDE.md

**File**: `docs/how-to/IMPORT_OPTIMIZER_GUIDE.md`

**Change**: Line 105
- Find: `use perl_parser::code_actions::{CodeActionsProvider, CodeActionKind};`
- Replace with: `use perl_lsp_code_actions::{CodeActionsProvider, CodeActionKind};`

**Verify**:
```bash
grep -n "use perl_lsp_code_actions::" /h/Code/Rust/perl-lsp/docs/how-to/IMPORT_OPTIMIZER_GUIDE.md
```
Expected: `105:use perl_lsp_code_actions::{CodeActionsProvider, CodeActionKind};`

---

### Step 11: Update doc comment in implementation_provider.rs

**File**: `crates/perl-lsp/src/features/implementation_provider.rs`

**Change**: Line 54 (inside `/// # Examples` doc comment, within `rust,ignore` code block)
- Find: `/// use perl_parser::implementation_provider::ImplementationProvider;`
- Replace with: `/// use perl_lsp_navigation::ImplementationProvider;`

**Verify**:
```bash
grep -n "/// use perl_lsp_navigation::ImplementationProvider" /h/Code/Rust/perl-lsp/crates/perl-lsp/src/features/implementation_provider.rs
```
Expected: `54:    /// use perl_lsp_navigation::ImplementationProvider;`

---

## Compilation and Test Verification

After all file changes are complete, run the following verification suite in order:

### Verify 1: Check no LSP re-exports remain in parser
```bash
cargo tree -p perl-parser --edges normal | grep "perl-lsp-"
```
Expected: (empty output)

### Verify 2: Build parser library
```bash
cargo build -p perl-parser --release 2>&1
```
Expected: Build succeeds, no errors

### Verify 3: Build LSP server
```bash
cargo build -p perl-lsp-rs --release 2>&1
```
Expected: Build succeeds, no errors

### Verify 4: Run parser tests (including ast_snapshot_tests)
```bash
cargo test -p perl-parser 2>&1
```
Expected: All tests pass

### Verify 5: Run LSP tests
```bash
cargo test -p perl-lsp-rs 2>&1
```
Expected: All tests pass

### Verify 6: Lint parser crate
```bash
cargo clippy -p perl-parser --lib 2>&1
```
Expected: No warnings or errors

### Verify 7: Code formatting check
```bash
cargo xtask fmt --check 2>&1
```
Expected: All files are properly formatted

---

## Test File Location

The Red TDD builder should place failing tests in:
- `crates/perl-parser/tests/ast_snapshot_tests.rs` — existing test file that validates `semantic_tokens` import works after step 7
- No new test files needed; existing tests validate the refactor

The builder should write assertions that verify:
1. `semantic_tokens::legend()` still works at lines 265, 272, 280
2. All LSP provider types are no longer accessible via `perl_parser::{code_actions, completion, ...}`
3. LSP functionality still works when imported directly from provider crates

---

## Sign-off Commands

After implementation and before marking `green-tdd-reviewed`:

```bash
rtk cargo test --workspace --lib 2>&1 | grep -E "test result|FAILED"
rtk cargo clippy --workspace --lib 2>&1 | grep -E "warning|error" | head -10
rtk cargo xtask fmt --check 2>&1
rtk gh pr create --title "refactor(parser): remove LSP provider re-exports (#4414)" \
  --body "Restores parser as pure leaf crate by removing LSP provider dependencies and re-exports."
```

Expected: All green, PR created.

---

## Notes

- **Scope**: 11 files changed (1 Cargo.toml, 3 src files in parser, 2 src files in lsp, 1 test file, 4 doc files)
- **Deletions**: 8 dependencies, 60 lines of re-export code, 1 entire file (tooling.rs)
- **Additions**: 3 import statements in live code, 9 import fixes in doc examples
- **Deferred**: `crates/perl-parser/src/ide/lsp_compat/` (5084 LOC) — follow-up issue per ADR-0041
- **Risk**: Low — pure removal refactor, all call sites verified

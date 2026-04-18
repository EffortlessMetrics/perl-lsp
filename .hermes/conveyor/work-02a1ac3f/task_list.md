# Task List: Add LSP Support for async/await Keywords

## Phase 1: Keyword Lists (perl-lexer)
- [ ] Add `await` to `KEYWORDS` array in `crates/perl-lexer/src/keywords/mod.rs` (alphabetically sorted)
- [ ] Add `async` to `KEYWORDS` array in `crates/perl-lexer/src/keywords/mod.rs` (alphabetically sorted)
- [ ] Add `await` to `LSP_COMPLETION_KEYWORDS` array (alphabetically sorted)
- [ ] Add `async` to `LSP_COMPLETION_KEYWORDS` array (alphabetically sorted)
- [ ] Add `await` to `LEXER_KEYWORDS` array (alphabetically sorted)
- [ ] Add `async` to `PARSER_LSP_KEYWORDS` array (alphabetically sorted)
- [ ] Run `cargo test -p perl-lexer -- keywords` to verify sorted+unique

## Phase 2: Completion Documentation (perl-lsp-completion)
- [ ] Add `async` case to `keyword_doc()` in `crates/perl-lsp-completion/src/completion/keywords.rs`
- [ ] Add `await` case to `keyword_doc()` in `crates/perl-lsp-completion/src/completion/keywords.rs`
- [ ] Run `cargo test -p perl-lsp-completion` to verify

## Phase 3: Semantic Tokens (perl-lsp-semantic-tokens)
- [ ] Add `"await"` to hardcoded keyword match arm in `crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs` (~line 452-457)
- [ ] Emit `keyword` token type (13) for `NodeKind::Unary { op: "await" }` nodes
- [ ] Run `cargo test -p perl-lsp-semantic-tokens` to verify
- [ ] Run `cargo test -p perl-parser-core -- fix_async_await` to verify no regression

## Phase 4: Verification
- [ ] Run full test suite: `cargo test -p perl-semantic-analyzer -- async`
- [ ] Verify `async { }` still parses as function call (not keyword) — existing tests in `fix_async_await_3608.rs`
- [ ] Verify `await::foo()` still parses as function call — existing tests cover this

## Deferred: async Semantic Tokens (Future Work)
- [ ] Track `async_span` in `NodeKind::Subroutine` (requires AST changes)
- [ ] Emit semantic tokens for `async` keyword from AST layer (not lexer layer)
- [ ] This is a separate work item — not in scope for #3538

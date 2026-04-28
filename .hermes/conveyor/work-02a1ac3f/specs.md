# Specification: LSP Support for Perl 5.36+ async/await Keywords

## Feature Summary

Add LSP support for Perl 5.36+ experimental `async` and `await` keywords (via `use feature 'async_await'`). This enables:
- Keyword completion in the IDE
- Hover documentation for `async` and `await`
- Semantic token highlighting for `await` (via `keyword` token type)

This does **not** include semantic token highlighting for `async` (requires AST span tracking changes beyond this work item's scope).

## Acceptance Criteria

### AC1: Keyword Completion
**Given** a Perl file with `use v5.36; use feature 'async_await';`
**When** the user triggers completion at a position where `async` or `await` would be valid
**Then** the LSP offers `async` and `await` as completion items with documentation

*Verification*: `cargo test -p perl-lsp-completion -- async` (or equivalent)

### AC2: Hover Documentation
**Given** a Perl file with `use v5.36; use feature 'async_await';`
**When** the user hovers over the `async` keyword or `await` keyword
**Then** the LSP returns documentation explaining:
- `async`: Marks a subroutine as asynchronous (Perl 5.36+ experimental feature, requires `use feature 'async_await'`)
- `await`: Suspends execution until a Future completes (Perl 5.36+ experimental feature)

*Verification*: `keyword_doc()` function in `perl-lsp-completion/src/completion/keywords.rs` has cases for `async` and `await`

### AC3: Semantic Token Highlighting for `await`
**Given** a Perl file with `use v5.36; use feature 'async_await';`
**When** the file contains an `await` expression (e.g., `await $future`)
**Then** the LSP emits a semantic token with type `keyword` (13) for the `await` token

*Verification*: `cargo test -p perl-lsp-semantic-tokens` passes; `fix_async_await_3608.rs` tests still pass

### AC4: Existing Tests Pass
**Given** all existing async/await related tests in the codebase
**When** this change is integrated
**Then** all existing tests continue to pass (no regression in parser, semantic analyzer, or other keyword functionality)

*Verification*: `cargo test -p perl-parser-core -- fix_async_await` and `cargo test -p perl-semantic-analyzer -- async`

## Non-Goals

1. **Semantic token highlighting for `async`**: The parser stores `async` as a string in `NodeKind::Subroutine { attributes: Vec<String> }` without source span tracking. Emitting semantic tokens for the `async` keyword requires AST changes to record `async_span` — this is deferred to a follow-up work item.

2. **New AST nodes**: `await` stays as `NodeKind::Unary { op: "await" }`; no new `AwaitExpression` node type.

3. **Type inference for async functions**: No type inference, scope analysis, or framework-specific analysis beyond what already exists.

4. **Code actions**: No code actions, refactoring, or diagnostics for async/await.

5. **Changes to hover provider**: Hover documentation comes only from `keyword_doc()`; no changes to the hover provider itself.

## Dependencies

1. **Perl 5.36+**: The feature is only meaningful with `use feature 'async_await'` and Perl 5.36+
2. **Keyword lists**: Changes target `crates/perl-lexer/src/keywords/mod.rs` (confirmed correct path, not `perl-keywords`)
3. **Semantic token framework**: Uses existing `keyword` token type (13) and existing `async` modifier (bit 6) in `perl-lsp-semantic-tokens/src/semantic_tokens.rs`
4. **keyword_doc() framework**: Uses existing `keyword_doc()` function in `perl-lsp-completion/src/completion/keywords.rs`

## Implementation Notes

### Keyword List Changes (Phase 1)
File: `crates/perl-lexer/src/keywords/mod.rs`

Add `async` and `await` to:
- `KEYWORDS` (sorted alphabetically)
- `LSP_COMPLETION_KEYWORDS` (sorted alphabetically)
- `PARSER_LSP_KEYWORDS` (sorted alphabetically)

Add `await` only (NOT `async`) to:
- `LEXER_KEYWORDS` (sorted alphabetically)

**Rationale**: `async` cannot go into `LEXER_KEYWORDS` because the parser treats `async { }` as a function call. Adding `async` there would cause incorrect semantic token emission.

### Completion Documentation (Phase 2)
File: `perl-lsp-completion/src/completion/keywords.rs`

Add cases to `keyword_doc()`:
```rust
"async" => Some("Marks a subroutine as asynchronous (Perl 5.36+ experimental, requires `use feature 'async_await'`)"),
"await" => Some("Suspends execution until a Future completes (Perl 5.36+ experimental)"),
```

### Semantic Tokens (Phase 3)
File: `perl-lsp-semantic-tokens/src/semantic_tokens.rs`

1. Add `"await"` to hardcoded keyword match arm at ~line 452-457 (does NOT use `is_lexer_keyword()`)
2. Emit `keyword` token type (13) for `NodeKind::Unary { op: "await" }` nodes

**Note**: `async` semantic tokens are **deferred** — the parser does not track `async_span`.

### Verification Commands
```bash
cargo test -p perl-lexer -- keywords     # keyword list tests (sorted+unique)
cargo test -p perl-lsp-completion        # completion tests
cargo test -p perl-lsp-semantic-tokens   # semantic token tests
cargo test -p perl-parser-core -- fix_async_await  # parser tests
cargo test -p perl-semantic-analyzer -- async       # semantic analyzer tests
```

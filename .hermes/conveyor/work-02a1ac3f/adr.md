# ADR 3538: LSP Support for Perl 5.36+ async/await Keywords

## Status
Proposed

## Context

GitHub issue #3538 requests LSP support for Perl 5.36+ experimental `async` and `await` keywords (via `use feature 'async_await'`). These keywords are currently absent from all keyword lists in the perl-lsp codebase:

- `crates/perl-lexer/src/keywords/mod.rs` — the single source of truth for keyword definitions
- Keyword lists: `KEYWORDS`, `LSP_COMPLETION_KEYWORDS`, `LEXER_KEYWORDS`, `PARSER_LSP_KEYWORDS`

This absence means:
1. The lexer does not recognize `async`/`await` as keywords
2. LSP completion does not offer `async`/`await` as completions
3. `keyword_doc()` in perl-lsp-completion has no documentation for these keywords
4. Semantic tokens do not highlight `async`/`await`

The tree-sitter grammar and native parser already handle these keywords correctly — `await` parses as `NodeKind::Unary { op: "await" }` and `async` parses as an attribute on `Subroutine`.

## Decision

### Keyword List Updates (per keyword type)

| Keyword | KEYWORDS | LSP_COMPLETION_KEYWORDS | LEXER_KEYWORDS | PARSER_LSP_KEYWORDS |
|---------|----------|--------------------------|----------------|---------------------|
| `await` | ✅ Add   | ✅ Add                   | ✅ Add         | ✅ Add              |
| `async` | ✅ Add   | ✅ Add                   | ❌ Do NOT add  | ✅ Add              |

**Rationale for not adding `async` to `LEXER_KEYWORDS`**: The native parser treats `async { }` as a function call (block as first argument), not as a keyword. Adding `async` to `LEXER_KEYWORDS` would cause the lexer to emit a `Keyword` token for what the parser semantically parses as a function call, resulting in incorrect semantic token emission.

**Rationale for adding `await` to `LEXER_KEYWORDS`**: `await` is unambiguous — `await::foo()` is parsed as a function call (the `::` makes it a qualified function), while bare `await` is always the keyword. This is context-safe.

### Semantic Token Emission (Phase 3)

1. **`await`**: Add to hardcoded keyword match arm in `perl-lsp-semantic-tokens/src/semantic_tokens.rs:452-457` (independent of `LEXER_KEYWORDS` lookup). Emit `keyword` token type (13) for `NodeKind::Unary { op: "await" }`.

2. **`async`**: **Deferred**. The parser stores `async` as a string in `NodeKind::Subroutine { attributes: Vec<String> }` without source span tracking. Emitting semantic tokens for `async` requires AST changes to record `async_span`. This is a separate work item.

### Completion Documentation (Phase 2)

Add `async` and `await` cases to `keyword_doc()` in `perl-lsp-completion/src/completion/keywords.rs`.

## Consequences

### Benefits
- Minimal, targeted changes leveraging existing keyword infrastructure
- Completion support for `async`/`await` keywords
- `await` gets proper semantic token highlighting
- `await` gets `keyword_doc()` documentation
- Low risk: `await` addition to `LEXER_KEYWORDS` is context-safe
- No AST changes required for Phase 1-2

### Tradeoffs / Risks
1. **Experimental feature**: Perl's `async_await` is experimental; documentation must note this
2. **`async` semantic tokens deferred**: Without AST span tracking for the `async` attribute, Phase 3 cannot emit semantic tokens for `async` — only `await`
3. **Sort order required**: All keyword lists use binary search and must remain alphabetically sorted
4. **Context-sensitivity of `async`**: The lexer cannot safely emit `async` as a keyword token (conflicts with function-call parsing), so `async` only gets completion + documentation, not tokenization or semantic highlighting

## Alternatives Considered

### Alternative 1: Add `async` to `LEXER_KEYWORDS` (rejected)
The original plan proposed adding `async` to `LEXER_KEYWORDS`. However, plan review identified that `async { }` is parsed as a function call by the native parser. Adding `async` to `LEXER_KEYWORDS` would cause the lexer to emit `Keyword` tokens that the semantic token emitter would then incorrectly highlight, creating a mismatch between lexer and parser semantics.

### Alternative 2: Create dedicated AST nodes (rejected)
An alternative approach would create `NodeKind::AwaitExpression` and track `async_span` on `Subroutine`. This was rejected because:
- `await` as `Unary { op: "await" }` is semantically correct (unary operator)
- `async` as attribute is how the parser already handles it
- AST changes are higher risk and scope (belong in a follow-up work item)
- The current approach still delivers meaningful LSP support incrementally

### Alternative 3: No changes to `LEXER_KEYWORDS` at all (rejected)
If we don't add `await` to `LEXER_KEYWORDS`, the semantic token emitter cannot use `is_lexer_keyword()` to emit tokens. However, the semantic token emitter has its own hardcoded keyword list, so `await` could still be supported there. But adding `await` to `LEXER_KEYWORDS` is the cleaner path and consistent with how other keywords work.

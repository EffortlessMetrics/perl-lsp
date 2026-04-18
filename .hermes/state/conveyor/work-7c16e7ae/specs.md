# Specification — work-7c16e7ae

## Feature

Add Perl snippets for modern syntax and frameworks to both VS Code native snippets and LSP completion snippets, and add missing modern Perl keywords to `LSP_COMPLETION_KEYWORDS`.

## Motivation

GitHub issue #3431 reports that the VS Code extension's `snippets/perl.json` lacks snippets for modern Perl patterns. Investigation revealed the LSP completion snippet system has identical gaps. Additionally, modern Perl keywords (`class`, `method`, `field`, `defer`, `given`, `when`, `catch`, `finally`, `say`) are recognized by the lexer but not surfaced in LSP keyword completion.

## Scope

### In Scope

1. **VS Code snippets** (`vscode-extension/snippets/perl.json`):
   - Modern Perl 5.38+ class syntax: `class`, `method`, `field`
   - Perl 5.36+ defer: `defer`
   - Perl 5.10+ given/when: `given`, `when`
   - Perl 5.34+ try/catch/finally: `catch`, `finally` (NOTE: `try` already exists)
   - Moo/Moose method modifiers: `around`, `before`, `after`, `with`, `has-builder`, `has-lazy`
   - Test::More extensions: `skip`, `todo`, `bail`, `plan`, `throws`
   - DBI patterns: `db-connect`, `db-prepare`, `db-transaction`
   - Common idioms: `slurp`, `say`, `state`, `slurp-file`, `read-lines`

2. **LSP snippets** (`crates/perl-lsp-completion/src/completion/snippets.rs`):
   - Same categories as VS Code, using descriptive trigger names (e.g., `perlclass`, `perlmethod`)
   - Avoid `trycatch` trigger since VS Code `try` prefix would conflict via LSP

3. **Keywords** (`crates/perl-lexer/src/keywords/mod.rs`):
   - Add `catch`, `class`, `defer`, `field`, `finally`, `given`, `method`, `say`, `when` to `LSP_COMPLETION_KEYWORDS`
   - Must remain alphabetically sorted
   - Activate dead `keyword_doc()` entries for `given`, `when`, `say`

### Out of Scope

- Parser or semantic analyzer changes (already supports 5.38+ features)
- VS Code `package.json` snippet registration changes
- Other editors' snippet configurations (Emacs, Neovim) — they use LSP or their own config
- Moose/Moo `has` option key completions (already exists)
- Test::Most or other test modules beyond Test::More

## Acceptance Criteria

1. **Keyword Completions**: Typing `class `, `method `, `field `, `defer `, `given `, `when `, `catch `, `finally `, or `say ` in an LSP client triggers keyword completion suggestions.

2. **VS Code Snippet Triggers**: All VS Code snippet prefixes listed in Scope section are present in `vscode-extension/snippets/perl.json` with valid JSON structure (prefix, body, description fields). No duplicate triggers exist.

3. **LSP Snippet Triggers**: All LSP snippet triggers listed in Scope section are present in `crates/perl-lsp-completion/src/completion/snippets.rs`. No duplicate triggers exist within the LSP system or with VS Code.

4. **Keyword Ordering**: `LSP_COMPLETION_KEYWORDS` array remains alphabetically sorted after additions. `cargo test -p perl-lexer` passes.

5. **No Try-Catch Duplication**: LSP does NOT add a `trycatch` trigger — VS Code already has `try` prefix and duplication would cause confusion.

6. **Perl Version Documentation**: Snippets for version-gated features (class, method, field = 5.38+; defer = 5.36+; given/when = 5.10+; catch/finally = 5.34+) include version requirements in their documentation.

7. **Tests Pass**: `cargo test -p perl-lsp-completion` and `cargo test -p perl-lexer` pass without modification to test files.

## Dependencies

- Perl parser already supports 5.38+ class syntax (verified in `test_corpus/modern_perl_features.pl`)
- `keyword_doc()` function in `keywords.rs` already has entries for `given`, `when`, `default`, `say` (dead entries awaiting keyword activation)
- VS Code snippet JSON structure validated by `vscode_snippet_library_tests.rs`
- LSP snippet uniqueness validated by `no_duplicate_triggers` test

## Non-Goals

- This does NOT add completion for Moose/Moo `has` option keys (already exists)
- This does NOT change the VS Code extension's `package.json` snippet registration
- This does NOT add snippets for non-Perl files (PHP, JavaScript, etc.)
- This does NOT modify the parser or semantic analyzer

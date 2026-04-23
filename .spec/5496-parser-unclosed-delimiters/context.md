# Context: #5496

## Decision log

**Decision:** Extend existing error_recovery_tests.rs inline (Option 3) rather than creating a new file
- **Rationale:** Parser error recovery tests are organized in a single file by design; adding to this file keeps tests discoverable and maintainable. New test file would fragment related tests.
- **Alternative rejected:** Create `delimiter_recovery_tests.rs` — would require parallel test infrastructure, harder to maintain parity with statement-level recovery patterns.

**Decision:** Test only unclosed delimiters, not closed-but-invalid delimiters
- **Rationale:** Issue scope covers unclosed cases (missing closing bracket/paren/quote). Invalid syntax within properly closed delimiters is handled by lexer, not parser error recovery.
- **Scope boundary:** qw(), q{}, qq(), qr// unclosed; not qw(stuff) followed by syntax error inside.

**Decision:** Use simple code snippets, no complex nesting
- **Rationale:** Tests validate recovery mechanism, not Perl semantics. Simple examples make assertions clearer and errors easier to debug.

## Objections addressed

**Concern:** Parser behavior for unclosed delimiters is undefined
- **Resolution:** Issue notes that parser's error handling assumes delimiters close; tests validate it doesn't panic. If parser needs to change recovery strategy, tests become acceptance tests for new behavior.

**Concern:** Some delimiters (qw, q, qq) may be handled differently (lexer vs parser)
- **Resolution:** Error recovery tests run at parser level. If lexer already consumed and reported error, parser recovery tests validate parser doesn't panic on resulting token stream. Either way, tests validate end-to-end behavior.

## Research findings

**Confirmed facts:**
- File `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs` exists and has 493 lines (as of this spec)
- Existing tests use pattern: `let mut parser = Parser::new(code); let result = parser.parse()`
- Parser exports `errors()` method to collect recovery errors
- `perl_tdd_support::must()` is available for unwrapping Result in tests
- No prior tests cover unclosed delimiter cases (lines 220-270 cover patterns, but only valid syntax)

**Parser error recovery architecture:**
- Phase 2 recovery: parser attempts to recover and produces structured nodes (VariableDeclaration, not raw Error)
- Error list is accumulated in parser state, accessible via errors() method
- Successful recovery allows parser.parse() to return Ok() even with errors

## Related issues

- #3500 — String interpolation with incomplete variable (related: incomplete escape sequences in unclosed strings)
- blockers.yaml line 55-61 — unclosed_paren category (related: paren handling in qw and delimiters)
- #5496 — This issue (new coverage for error recovery)

## Test pattern consistency

Tests follow existing pattern from error_recovery_tests.rs:
1. Create Parser with error code
2. Call parse() and check result.is_ok()
3. Unwrap AST with must() helper
4. Pattern match on NodeKind::Program to inspect statements
5. Assert parser.errors().is_empty() == false
6. Include descriptive assertion messages

This consistency ensures tests are maintainable and aligned with codebase conventions.

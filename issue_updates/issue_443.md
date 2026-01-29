# Issue #443: Parser Timeout Risk: Recursive descent limit

## Status
**Resolved / Verified**

## Analysis
The issue required implementing a recursion depth limit and timeout for heredoc parsing to prevent hangs.
Reviewing the codebase, I confirmed that `MAX_HEREDOC_DEPTH` is set to 100 and `HEREDOC_TIMEOUT_MS` is 5000ms in both `crates/perl-lexer/src/lib.rs` and `crates/tree-sitter-perl-rs/src/heredoc_parser.rs`.

## Fixes & Verification
1. Verified `perl-lexer` implements the budget guards (depth and timeout).
2. `cargo test -p perl-lexer` passed, including `tests/heredoc_security_tests.rs`.
3. Fixed `xtask` tests which were incorrectly asserting `MAX_HEREDOC_DEPTH` was 10 (updated to 100).
4. Fixed `xtask` tests for nodekind analysis which were looking for non-existent node kinds.
5. Ran `cargo test -p xtask` and it passed.

## Recommendation
Close the issue. The protections are in place and verified by tests.

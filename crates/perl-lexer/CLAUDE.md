# perl-lexer

Context-aware tokenizer. Handles Perl's complex lexical grammar.

## Key Challenges
- Context-sensitive tokenization (/ is divide or regex depending on context)
- Special variables ($_, $!, $$, $^W, etc.)
- Quote-like operators (q//, qq//, qw//, etc.)

## Verify
```bash
cargo fmt --all
cargo clippy -p perl-lexer --tests
cargo test -p perl-lexer
```

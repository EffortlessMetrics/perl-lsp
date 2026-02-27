# CLAUDE.md -- perl-ts-logos-lexer

## Crate Overview

- **Version**: 0.1.0
- **Tier**: Tree-sitter microcrate (experimental, `publish = false`)
- **Purpose**: Logos-based tokenizer and recursive descent parsers for Perl, providing context-aware lexing with slash disambiguation and regex/quote-like operator parsing.

## Commands

```bash
cargo build -p perl-ts-logos-lexer
cargo test -p perl-ts-logos-lexer
cargo clippy -p perl-ts-logos-lexer
cargo doc -p perl-ts-logos-lexer --no-deps

# Build with optional logos-tokens feature (enables logos_lexer + token_parser modules)
cargo build -p perl-ts-logos-lexer --features logos-tokens
cargo test -p perl-ts-logos-lexer --features logos-tokens
```

## Architecture

### Dependencies

- **logos** (0.16) -- Lexer generator, drives token definitions in `simple_token` and `logos_lexer`
- **regex** (1.x) -- Used for pattern support
- **chumsky** (0.12) -- Parser combinator library, used by `token_parser` (behind `logos-tokens` feature)
- **perl-parser-pest** -- Provides AST types consumed by `token_parser`
- **perl-tdd-support** (dev) -- Test helpers (`must`, `must_some`)

### Key Types and Modules

| Module | Public Types | Description |
|--------|-------------|-------------|
| `simple_token` | `Token`, `PerlLexer` | Simplified Logos token enum; standalone context-aware lexer |
| `context_lexer_simple` | `ContextLexer`, `SlashContext` | Wraps `simple_token::Token` logos lexer with `/` disambiguation |
| `context_lexer_v2` | `ContextLexerV2`, `EnhancedToken`, `SlashContext` | Enhanced lexer returning `EnhancedToken` with parsed regex data |
| `regex_parser` | `RegexParser`, `QuoteConstruct`, `QuoteOperator` | Parses regex and quote-like operators (`m//`, `s///`, `qr//`, etc.) |
| `token_ast` | `AstNode` | Simple AST node with type, span, optional value, and children |
| `simple_parser` | `SimpleParser` | Recursive descent parser using `ContextLexer`, produces `token_ast::AstNode` |
| `simple_parser_v2` | `SimpleParser` | Extended parser with full operator precedence, method calls, array/hash literals |
| `logos_lexer` | `Token`, `PerlLexer`, `LexerMode` | Feature-gated full Logos token enum with keyword callback dispatch |
| `token_parser` | `TokenParser` | Feature-gated Chumsky parser producing `perl_parser_pest::AstNode` |

### Feature Flags

- **`logos-tokens`** -- Enables `logos_lexer` and `token_parser` modules (full token enum + Chumsky parser)

## Usage Examples

```rust
// Context-aware lexing with slash disambiguation
use perl_ts_logos_lexer::context_lexer_simple::ContextLexer;
let mut lexer = ContextLexer::new("$x = 10 / 2 + /test/");
while let Some(token) = lexer.next() {
    // Token::Divide for division, Token::Regex for regex
}

// Regex/quote-like operator parsing
use perl_ts_logos_lexer::regex_parser::RegexParser;
let mut parser = RegexParser::new("s/old/new/g", 1);
let result = parser.parse_substitute_operator(); // Ok(QuoteConstruct { ... })

// Recursive descent parsing
use perl_ts_logos_lexer::simple_parser::SimpleParser;
let mut parser = SimpleParser::new("my $x = 42;");
let ast = parser.parse(); // Result<AstNode, String>
```

## Important Notes

- The `simple_parser` and `simple_parser_v2` modules both export `SimpleParser` -- they are in separate modules and do not conflict, but imports must be qualified.
- Slash disambiguation relies on tracking whether the parser expects an operand or operator after the previous token (`SlashContext`).
- The `logos_lexer` module uses a callback on the `Identifier` regex to dispatch keywords, returning distinct token variants for each Perl keyword.
- This crate is `publish = false` and experimental; it is not part of the stable public API.

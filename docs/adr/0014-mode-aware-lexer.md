# ADR-0014: Mode-Aware Lexer for Slash Disambiguation

**Status**: Accepted
**Date**: 2025-01-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [SLASH_DISAMBIGUATION.md](../explanation/SLASH_DISAMBIGUATION.md), [mode.rs](../../crates/perl-lexer/src/mode.rs)

## Context

Perl's use of the slash character (`/`) for multiple purposes creates one of the most challenging context-sensitive parsing problems in programming language implementation. The same character can represent:

1. **Division operator**: `$x / 2`
2. **Regex delimiter**: `/pattern/`
3. **Substitution operator**: `s/pattern/replacement/`
4. **Transliteration**: `tr/abc/xyz/`
5. **Quote-regex**: `qr/pattern/`

### Problem Statement

1. **Context Sensitivity**: The interpretation of `/` depends on preceding tokens
2. **Grammar Complexity**: Context-free grammars cannot express this ambiguity
3. **Parser Dependencies**: Traditional solutions require C-based parsers (perl itself)
4. **Pure Rust Goal**: Project aims to avoid C dependencies for security and portability

### Ambiguity Examples

| Expression | First `/` Meaning | Context |
|------------|------------------|---------|
| `$x / 2` | Division | After identifier |
| `if (/foo/)` | Regex start | After keyword |
| `1/ /abc/` | Division, then regex | After number, then operator |
| `split /,/, $x` | Regex start | After function name |
| `s/a/b/` | Substitution | After `s` |

## Decision

**We implement a mode-aware lexer with state machine tracking (`ExpectTerm`/`ExpectOperator`) combined with a preprocessing adapter that substitutes Unicode alternatives for ambiguous tokens.**

### Lexer Mode State Machine

```rust
/// Lexer modes for context-sensitive parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
    /// Expecting a delimiter for quote-like operators
    ExpectDelimiter,
    /// Inside a format declaration body
    InFormatBody,
    /// Inside __DATA__ or __END__ section
    InDataSection,
}
```

### Mode Transition Heuristics

| Previous Token | Next Mode | Example |
|----------------|-----------|---------|
| identifier | ExpectOperator | `$x / 2` |
| number | ExpectOperator | `10 / 3` |
| closing paren/bracket | ExpectOperator | `) / 2` |
| keyword | ExpectTerm | `if /pattern/` |
| operator | ExpectTerm | `=~ /test/` |
| opening paren/bracket | ExpectTerm | `( /regex/` |

### Preprocessing Adapter

To integrate with context-free grammar parsers, we preprocess ambiguous tokens:

| Original | Replacement | Unicode | Purpose |
|----------|-------------|---------|---------|
| `/` (division) | `÷` | U+00F7 | Disambiguate division |
| `s` (substitution) | `ṡ` | U+1E61 | Mark substitution context |
| `tr` (transliteration) | `ṫr` | U+1E6B | Mark transliteration context |
| `qr` (quote-regex) | `ǫr` | U+01EB | Mark quote-regex context |

### Grammar Integration

```pest
// grammar.pest accepts both original and preprocessed tokens
multiplicative_op = { "*" | "/" | "÷" | "%" | "x" }
substitution = { ("s" | "ṡ") ~ ... }
transliteration = { ("tr" | "ṫr" | "y" | "ẏ") ~ ... }
qr_regex = { ("qr" | "ǫr") ~ ... }
```

### Postprocessing

After parsing, the AST is traversed to restore original operators:
- `÷` → `/` in binary operations
- Preprocessed markers removed from AST nodes

## Consequences

### Positive

1. **Pure Rust Implementation**: No C dependencies required for parsing
2. **Security**: Avoids C-based parser vulnerabilities (buffer overflows, memory safety)
3. **Portability**: Works on any platform Rust supports
4. **Maintainability**: All parsing logic in Rust, easier to debug and extend
5. **Performance**: ~10-20μs preprocessing overhead, no backtracking at parse time
6. **Completeness**: Handles all Perl slash ambiguity cases

### Negative

1. **Complexity**: Additional preprocessing pass required
2. **Unicode in Source**: Preprocessed tokens use Unicode characters (not visible in original source)
3. **Debugging**: Intermediate representation differs from source
4. **Grammar Coupling**: Grammar must accept both original and preprocessed forms

### Mitigations

- Postprocessing restores original tokens in AST
- Comprehensive test coverage validates all edge cases
- Clear documentation explains the transformation pipeline

## Implementation

### Core Files

| File | Purpose |
|------|---------|
| `crates/perl-lexer/src/mode.rs` | LexerMode enum and transitions |
| `crates/perl-lexer/src/lib.rs` | Mode-aware tokenization |
| `crates/perl-lexer/src/adapter.rs` | Preprocessing adapter |

### Usage Example

```rust
use tree_sitter_perl::disambiguated_parser::DisambiguatedParser;

// Complex expression with mixed slash usage
let perl_code = "print 1/ /abc/ + s/x/y/g";
let ast = DisambiguatedParser::parse(perl_code)?;

// Correctly parses as:
// print((1 / /abc/) + s/x/y/g)
```

## Test Coverage

The implementation handles all edge cases:

1. **Division after identifier**: `x / 2` → Division
2. **Regex after operator**: `=~ /foo/` → Regex
3. **Mixed expressions**: `1/ /abc/` → Division then Regex
4. **Substitution variants**: `s/a/b/`, `s{a}{b}`, `s'a'b'`
5. **Complex precedence**: `split /,/, $x / 3`

### Test Commands

```bash
# Run lexer mode tests
cargo test -p perl-lexer -- mode

# Run slash disambiguation tests
cargo test -p perl-parser -- slash

# Run full parser tests
cargo test -p perl-parser
```

## Limitations

This approach handles slash disambiguation completely within PEG parser constraints. Remaining Perl features requiring stateful parsing:

- Full heredoc content collection
- Some runtime-dependent constructs

## References

- [Slash Disambiguation Explanation](../explanation/SLASH_DISAMBIGUATION.md)
- [Lexer Mode Source](../../crates/perl-lexer/src/mode.rs)
- [Issue #422: Slash Disambiguation](https://github.com/EffortlessMetrics/perl-lsp/issues/422)

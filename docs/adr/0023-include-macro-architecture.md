# ADR-0023: include! Macro Architecture for Parser Composition

**Status**: Accepted
**Date**: 2025-02-20
**Decision Makers**: Perl LSP Architecture Team
**Related**: [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md)

## Context

The Perl parser requires tight coupling between components for performance-critical operations while maintaining code organization and maintainability. Traditional Rust module composition patterns present trade-offs:

1. **Separate Modules**: Clean separation but requires public APIs and potential indirection overhead
2. **Single Large File**: Fast compilation and tight coupling but poor code organization
3. **Inline Modules**: Good organization but still requires visibility management

The recursive descent parser in `perl-parser-core` needs:
- Direct access to internal parser state across parsing functions
- Zero overhead for frequently called parsing methods
- Clear code organization for the 11+ focused parsing domains (expressions, statements, declarations, etc.)
- Ability to share internal helper functions without making them public API

## Decision

**We use Rust's `include!` macro to compose parser logic from multiple focused files into a single compilation unit, enabling tight coupling for performance while maintaining logical code organization.**

### Implementation Pattern

The parser is organized into focused domain files that are composed at compile time:

```rust
// crates/perl-parser-core/src/engine/parser/mod.rs
mod expressions;    // Expression parsing (binary ops, unary, literals)
mod statements;     // Statement parsing (if, while, for, etc.)
mod declarations;   // Package/sub declarations
mod terms;          // Term parsing (variables, subs, etc.)
mod quotes;         // Quote-like operators (q, qq, qr, s, tr)
mod heredoc;        // Heredoc handling
mod recovery;       // Error recovery logic
mod precedence;     // Operator precedence climbing
mod patterns;       // Regex pattern parsing
mod blocks;         // Block and scope handling
mod trivia;         // Whitespace and comment handling

// Composition via include! for tight coupling
include!("expressions.rs");
include!("statements.rs");
include!("declarations.rs");
// ... etc
```

### File Organization

| File | Purpose | Size |
|------|---------|------|
| `expressions.rs` | Binary/unary ops, literals, precedence | ~800 LOC |
| `statements.rs` | Control flow, compound statements | ~600 LOC |
| `declarations.rs` | Package, sub, variable declarations | ~400 LOC |
| `terms.rs` | Variables, subroutine calls, barewords | ~500 LOC |
| `quotes.rs` | Quote-like operators (q, qq, qr, s, tr) | ~700 LOC |
| `heredoc.rs` | Heredoc content collection | ~300 LOC |
| `recovery.rs` | Error recovery and resynchronization | ~250 LOC |
| `precedence.rs` | Operator precedence tables | ~150 LOC |
| `patterns.rs` | Regex pattern parsing | ~350 LOC |
| `blocks.rs` | Block and scope management | ~200 LOC |
| `trivia.rs` | Whitespace/comment handling | ~150 LOC |

### Benefits of This Approach

1. **Zero Runtime Overhead**: All code compiles as if in a single file
2. **Direct State Access**: All parsing functions access `Parser` struct directly
3. **Internal Helper Sharing**: Helper functions can be used without `pub` visibility
4. **Logical Organization**: Each file has a clear, focused responsibility
5. **Easier Navigation**: Developers can find parsing logic by domain
6. **Code Review Friendly**: Changes are isolated to relevant files

## Consequences

### Positive

- **Performance**: No function call indirection overhead for internal parsing operations
- **Maintainability**: Clear file organization despite tight coupling requirements
- **Encapsulation**: Internal helpers remain internal, not exposed as public API
- **Compilation**: Single compilation unit can enable better optimization
- **Testing**: Can test internal functions via the composed module

### Negative

- **Compilation Time**: Large single compilation unit may slow incremental builds
- **IDE Support**: Some IDEs may have reduced symbol navigation for `include!` content
- **Convention Required**: Team must understand the pattern to maintain it properly
- **Circular Dependencies**: Must be careful to avoid circular includes between files

### Mitigations

- Use `cargo check` for fast iteration during development
- Document the pattern clearly in code comments and architecture guides
- Keep individual files focused and under 1000 LOC where possible
- Use `#[cfg(test)]` blocks within files for unit testing

## References

- [crates/perl-parser-core/src/engine/parser/mod.rs](../../crates/perl-parser-core/src/engine/parser/mod.rs) - Main parser module
- [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md) - Overall architecture
- [Rust include! macro documentation](https://doc.rust-lang.org/std/macro.include.html)

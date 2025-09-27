# ISSUE-147: Substitution Operator Parsing Incomplete

## Context
The Perl parser currently claims ~100% Perl 5 syntax coverage but has incomplete support for substitution operators (`s///`). This represents a gap between the claimed completeness and actual implementation, affecting core parser functionality and downstream LSP features.

**Current State**: Parser only handles the pattern portion of substitution operators
**Location**: `/crates/perl-parser/src/parser_backup.rs:3110`
**Priority**: High (impacts fundamental Perl language support)

## User Story
As a Perl developer using the LSP server, I want complete substitution operator parsing so that syntax highlighting, hover information, and code completion work correctly for `s///` constructs, enabling me to write and maintain Perl code with full IDE support.

## Acceptance Criteria

**AC1**: Parse replacement text portion of substitution operator
- Given a substitution like `s/old/new/`, the parser must extract and represent "new" as the replacement text
- Must handle escaped characters and backreferences (e.g., `s/(\w+)/prefix_$1_suffix/`)

**AC2**: Parse and validate modifier flags for substitution operators
- Given flags like `s/old/new/gi`, the parser must extract and validate "g" (global) and "i" (case-insensitive)
- Must support all valid Perl substitution flags: g, i, m, s, x, o, e, r
- Must reject invalid flag combinations where applicable

**AC3**: Handle alternative delimiter styles for substitution operators
- Given `s{old}{new}g`, `s|old|new|gi`, `s#old#new#`, the parser must correctly identify delimiters
- Must support any printable ASCII character as delimiter (excluding word characters)
- Must handle balanced delimiters: `()`, `{}`, `[]`, `<>`

**AC4**: Create proper AST representation for all substitution components
- AST must contain separate nodes/fields for: pattern, replacement, flags
- Must integrate with existing regex parsing for the pattern portion
- Must maintain source position information for all components

**AC5**: Add comprehensive test coverage for substitution operator variations
- Must include tests for basic forms: `s/pattern/replacement/flags`
- Must include tests for alternative delimiters and edge cases
- Must include tests for complex replacements with backreferences
- Must include negative tests for malformed substitution operators

**AC6**: Update documentation to reflect complete substitution support
- Update parser documentation to accurately reflect substitution operator support
- Remove or update any TODO comments related to incomplete substitution parsing
- Ensure consistency with the claimed ~100% Perl syntax coverage

## Technical Requirements

**Substitution Operator Structure:**
1. `s` keyword
2. Delimiter character (any printable ASCII except word characters)
3. Pattern (regex) - already supported
4. Same delimiter
5. Replacement text (can contain backreferences like `$1`, `$2`, `$&`, etc.)
6. Same delimiter
7. Optional modifier flags

**Examples That Must Work:**
```perl
s/old/new/g;                    # Global replacement
s/pattern/replacement/i;        # Case insensitive
s{old}{new}g;                  # Curly brace delimiters
s|old|new|gi;                  # Pipe delimiters with multiple flags
s/(\w+)/prefix_$1_suffix/;     # Backreferences in replacement
s#path/to/file#/new/path#;     # Hash delimiters for paths
```

## Impact Assessment
- **Parser Completeness**: Closes gap in claimed ~100% Perl syntax coverage
- **LSP Features**: Enables proper syntax highlighting and hover for `s///` constructs
- **User Experience**: Provides complete IDE support for fundamental Perl language feature
- **Code Quality**: Improves accuracy of parser's Perl language support claims

## Open Questions
None - requirements are well-defined from the GitHub issue and Perl language specification.
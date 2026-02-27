# Issue: Ambiguous Slash Division vs Regex

## Problem Description

### What We Found

The slash `/` character has dual meaning in Perl:
1. **Division operator**: `$a / $b` - divides `$a` by `$b`
2. **Regex delimiter**: `/pattern/` - matches a regex pattern

This creates a **P0 critical parsing ambiguity**:
- No clear disambiguation rules for slash context
- Parser may incorrectly parse division as regex
- No test coverage for ambiguous slash scenarios
- Potential for incorrect AST generation

This represents a **critical correctness and stability risk** that could cause incorrect parsing, incorrect semantic analysis, and wrong LSP diagnostics

### Minimal Reproduction

```perl
# Ambiguous slash - division vs regex
my $result = $a / $b;  # Division
my $match = $a =~ /$b/;  # Regex match

# Another ambiguous case
my $complex = $x / $y / $z;
my $regex = /$x/$y/$z/;
```

### Current Behavior

The parser may:
- Parse `/` as division in all contexts
- Parse `/` as regex delimiter in all contexts
- Not provide context-aware parsing
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Disambiguate slash based on context:
  - Division: `$a / $b` when used in arithmetic context
  - Regex: `/pattern/` when used in string matching or substitution
  - Provide clear error messages for ambiguous cases
- Generate correct AST structure
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with slash handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for ambiguous slash

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Context-aware slash parsing based on preceding/following tokens
   - [ ] Proper disambiguation rules for division vs regex
   - [ ] Clear error messages for ambiguous cases
   - [ ] Correct AST generation for both division and regex
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 6 test cases covering:
     - [ ] Simple division: `$a / $b`
     - [ ] Simple regex: `/pattern/`
     - [ ] Ambiguous arithmetic: `$x / $y / $z`
     - [ ] Ambiguous regex: `/pattern/` in arithmetic context
     - [ ] Nested expressions with multiple slashes
     - [ ] Edge cases: empty strings, whitespace
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for ambiguous slash usage
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for both division and regex
   - [ ] Code completion suggests appropriate slash usage

4. **Documentation**:
   - [ ] Slash disambiguation rules documented
   - [ ] Examples of ambiguous cases and resolution
   - [ ] API documentation updated with slash handling

### Solution Options

#### Option 1: Context-Aware Parsing (Recommended)

**Pros**:
- Comprehensive solution covering all cases
- Clear disambiguation rules
- Recommended approach

**Cons**:
- More complex implementation
- Requires significant parser changes

**Implementation**:
```rust
// Add context tracking to parser
enum SlashContext {
    Arithmetic,
    Regex,
    Unknown,
}

// Determine slash context based on preceding tokens
fn determine_slash_context(tokens: &[Token]) -> SlashContext {
    // Look at previous token to determine context
    // If previous token is variable or number, likely division
    // If previous token is string or pattern, likely regex
}
```

#### Option 2: Conservative Fallback

**Pros**:
- Simpler implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all edge cases
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to division when ambiguous
fn parse_slash() -> Node {
    // Always parse as division unless clear regex context
    parse_division()
}
```

#### Option 3: Explicit Disambiguation

**Pros**:
- Clearer intent
- Easier to understand for users

**Cons**:
- Requires changes to Perl syntax
- May not be backward compatible

**Implementation**:
```perl
// Perl could add explicit operators like:
my $result = $a div $b;  # Explicit division
my $match = $a m/ $b;    # Explicit regex match
```

### Path Forward

**Recommended**: Option 1 (Context-Aware Parsing)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's actual parsing behavior

**Implementation Steps**:
1. Add `SlashContext` enum to track context
2. Implement context determination logic in lexer/parser
3. Update slash parsing to use context
4. Add test cases for ambiguous scenarios
5. Update LSP providers to handle new slash nodes
6. Document slash disambiguation rules
7. Validate with existing corpus and real-world code
8. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 2 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for slash/division disambiguation

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for slash/division disambiguation

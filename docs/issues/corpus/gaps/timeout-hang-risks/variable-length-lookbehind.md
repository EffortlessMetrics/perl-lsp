# Issue: Variable-Length Lookbehind Regex

## Problem Description

### What We Found

Perl's regex engine supports variable-length lookbehind:
- `(?<=pattern)` - positive lookbehind
- `(?<!pattern)` - negative lookbehind
- Variable-length patterns: `(?<=\d{1,3})` - matches 1-3 digits behind
- Complex patterns: `(?<=\w+\s+)` - matches word characters and whitespace behind

This creates a **P2 medium risk** for parser complexity:
- Complex lookbehind syntax parsing
- Potential for incorrect lookbehind length calculation
- No comprehensive test coverage for variable-length lookbehind
- Edge cases: nested lookbehind, mixed lookbehind/lookahead

### Minimal Reproduction

```perl
# Variable-length lookbehind regex
my $pattern = qr/(?<=\d{1,3})\d/;
my $match = $text =~ /(?<=\d{1,3})\d/;

# Negative lookbehind
my $negated = qr/(?<!\d{1,3})\d/;

# Complex lookbehind
my $complex = qr/(?<=\w+\s+)\w+/;
```

### Current Behavior

The parser may:
- Not correctly parse variable-length lookbehind syntax
- Not calculate lookbehind length correctly
- Not handle nested lookbehind correctly
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Correctly parse variable-length lookbehind syntax (`(?<=...)`, `(?<!...)`)
- Calculate lookbehind length correctly
- Handle nested lookbehind correctly
- Generate correct AST structure
- Provide clear error messages for invalid lookbehind
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with regex handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for lookbehind

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly parse variable-length lookbehind syntax (`(?<=...)`, `(?<!...)`)
   - [ ] Calculate lookbehind length correctly
   - [ ] Handle nested lookbehind correctly
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for invalid lookbehind
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple lookbehind: `(?<=\d{1,3})`
     - [ ] Negative lookbehind: `(?<!\d{1,3})`
     - [ ] Nested lookbehind: `(?<=(?<=\d)\w+)\w+`
     - [ ] Complex lookbehind patterns
     - [ ] Edge cases: empty lookbehind, invalid lookbehind
     - [ ] Error recovery: invalid lookbehind syntax
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for invalid lookbehind
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for lookbehind
   - [ ] Code completion suggests valid lookbehind patterns

4. **Documentation**:
   - [ ] Lookbehind parsing rules documented
   - [ ] Examples of lookbehind patterns
   - [ ] API documentation updated with lookbehind handling

### Solution Options

#### Option 1: Lookbehind Parser (Recommended)

**Pros**:
- Comprehensive solution covering all cases
- Clear error messages for invalid lookbehind
- Recommended approach

**Cons**:
- More complex implementation
- Requires lookbehind length calculation

**Implementation**:
```rust
// Add lookbehind parser
struct LookbehindParser {
    // Lookbehind parsing state
}

// Parse lookbehind syntax
fn parse_lookbehind() -> Node {
    // Parse (?<=...) or (?<!...)
}
```

#### Option 2: Regex-Based Matching

**Pros**:
- Simpler implementation
- Faster for simple cases

**Cons**:
- May not handle all edge cases
- Limited regex capabilities for complex lookbehind

**Implementation**:
```rust
// Use regex to match lookbehind syntax
let lookbehind_regex = Regex::new(r"\(\?<=([^)]*)\)")?;
```

#### Option 3: Conservative Fallback

**Pros**:
- Simplest implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all lookbehind patterns
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to literal string when lookbehind detected
fn parse_lookbehind() -> Node {
    // Parse as literal string
    parse_literal()
}
```

### Path Forward

**Recommended**: Option 1 (Lookbehind Parser)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's regex engine

**Implementation Steps**:
1. Implement lookbehind parser with length calculation
2. Add lookbehind parsing logic to lexer/parser
3. Handle nested lookbehind correctly
4. Add test cases for lookbehind patterns
5. Update LSP providers to handle new lookbehind nodes
6. Document lookbehind parsing rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 3 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for lookbehind parsing

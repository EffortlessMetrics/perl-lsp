# Issue: Complex Quote Operator Delimiters

## Problem Description

### What We Found

Perl's quote operators (`q`, `qq`, `qx`, `qw`, `qr`, `s`, `tr`, `y`) support complex delimiter patterns:
- Balanced delimiters: `q{}`, `q[]`, `q<>`, `q()`
- Paired delimiters: `q//`, `q##`, `q||`, `q!!`
- Mixed delimiters: `q{}`, `q[]`, `q<>`, `q()`
- Nested delimiters: `q{nested{delimiters}}`

This creates a **P2 medium risk** for parser complexity:
- Complex delimiter matching logic
- Potential for incorrect nesting detection
- No comprehensive test coverage for all delimiter combinations
- Edge cases: whitespace, comments, escaped delimiters

### Minimal Reproduction

```perl
# Complex quote operator delimiters
my $str1 = q{This is a {braced} string};
my $str2 = q[This is a [bracketed] string];
my $str3 = q<This is a <angled> string>;
my $str4 = q(This is a (parenthesized) string);

# Nested delimiters
my $nested = q{outer{inner{deep}}};

# Mixed delimiters
my $mixed = q{outer[inner<deep>]};
```

### Current Behavior

The parser may:
- Not correctly handle balanced delimiters
- Not properly detect nested delimiter levels
- Fail on mixed delimiter patterns
- Not handle escaped delimiters correctly
- Generate incorrect AST structure

### Expected Behavior

The parser should:
- Correctly match balanced delimiters (`{}`, `[]`, `<>`, `()`)
- Properly detect nested delimiter levels
- Handle mixed delimiter patterns correctly
- Handle escaped delimiters correctly
- Generate correct AST structure
- Provide clear error messages for unmatched delimiters

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with quote operator handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for quote operators

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly match balanced delimiters (`{}`, `[]`, `<>`, `()`)
   - [ ] Properly detect nested delimiter levels
   - [ ] Handle mixed delimiter patterns correctly
   - [ ] Handle escaped delimiters correctly
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for unmatched delimiters

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple balanced delimiters: `q{}`, `q[]`, `q<>`, `q()`
     - [ ] Nested delimiters: `q{outer{inner}}`
     - [ ] Mixed delimiters: `q{outer[inner]}`
     - [ ] Escaped delimiters: `q{escaped\} delimiter}`
     - [ ] Edge cases: empty strings, whitespace, comments
     - [ ] Error recovery: unmatched delimiters
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for delimiter mismatches
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for quote operators
   - [ ] Code completion suggests appropriate delimiter pairs

4. **Documentation**:
   - [ ] Quote operator delimiter rules documented
   - [ ] Examples of complex delimiter patterns
   - [ ] API documentation updated with quote operator handling

### Solution Options

#### Option 1: Stack-Based Delimiter Matching (Recommended)

**Pros**:
- Handles nested delimiters correctly
- Clear error messages for mismatches
- Recommended approach

**Cons**:
- More complex implementation
- Requires careful stack management

**Implementation**:
```rust
// Use stack to track delimiter nesting
fn parse_quote_operator() -> Node {
    let mut delimiter_stack = Vec::new();
    // Parse with stack-based matching
}
```

#### Option 2: Regex-Based Matching

**Pros**:
- Simpler implementation
- Faster for simple cases

**Cons**:
- May not handle all edge cases
- Limited regex capabilities for complex nesting

**Implementation**:
```rust
// Use regex to match delimiters
let delimiter_regex = Regex::new(r"q\{([^{}]*)\}")?;
```

#### Option 3: Recursive Parsing

**Pros**:
- Most flexible solution
- Handles all edge cases

**Cons**:
- Most complex implementation
- May cause stack overflow for deeply nested patterns

**Implementation**:
```rust
// Recursively parse nested delimiters
fn parse_nested_delimiters() -> Node {
    // Recursive parsing logic
}
```

### Path Forward

**Recommended**: Option 1 (Stack-Based Delimiter Matching)

**Rationale**:
- Handles nested delimiters correctly
- Provides clear error messages
- Recommended approach
- Balances complexity and correctness

**Implementation Steps**:
1. Implement delimiter stack in lexer/parser
2. Add delimiter matching logic for balanced pairs
3. Handle escaped delimiters correctly
4. Add test cases for complex delimiter patterns
5. Update LSP providers to handle new quote operator nodes
6. Document quote operator delimiter rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 3-5 days for implementation + 2 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for quote operator parsing

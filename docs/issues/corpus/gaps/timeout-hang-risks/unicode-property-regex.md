# Issue: Unicode Property Regex

## Problem Description

### What We Found

Perl's regex engine supports Unicode properties:
- `\p{Script=Latin}` - matches characters in Latin script
- `\p{Category=Letter}` - matches letter characters
- `\p{Block=Basic_Latin}` - matches characters in Basic Latin block
- `\P{...}` - negated Unicode property matches

This creates a **P2 medium risk** for parser complexity:
- Complex Unicode property syntax
- Potential for incorrect property parsing
- No comprehensive test coverage for Unicode properties
- Edge cases: nested properties, negated properties

### Minimal Reproduction

```perl
# Unicode property regex
my $pattern = qr/\p{Script=Latin}/;
my $match = $text =~ /\p{Script=Latin}/;

# Negated Unicode property
my $negated = qr/\P{Category=Letter}/;

# Complex Unicode property
my $complex = qr/\p{Script=Latin}+\p{Category=Letter}/;
```

### Current Behavior

The parser may:
- Not correctly parse Unicode property syntax
- Not handle nested properties correctly
- Not support negated properties
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Correctly parse Unicode property syntax (`\p{...}`, `\P{...}`)
- Handle nested properties correctly
- Support negated properties
- Generate correct AST structure
- Provide clear error messages for invalid properties
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with regex handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for Unicode properties

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly parse Unicode property syntax (`\p{...}`, `\P{...}`)
   - [ ] Handle nested properties correctly
   - [ ] Support negated properties
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for invalid properties
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple Unicode property: `\p{Script=Latin}`
     - [ ] Negated Unicode property: `\P{Category=Letter}`
     - [ ] Nested properties: `\p{Script=Latin}+\p{Category=Letter}`
     - [ ] Complex Unicode property patterns
     - [ ] Edge cases: empty properties, invalid properties
     - [ ] Error recovery: invalid Unicode property syntax
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for invalid Unicode properties
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for Unicode properties
   - [ ] Code completion suggests valid Unicode properties

4. **Documentation**:
   - [ ] Unicode property parsing rules documented
   - [ ] Examples of Unicode property patterns
   - [ ] API documentation updated with Unicode property handling

### Solution Options

#### Option 1: Unicode Property Parser (Recommended)

**Pros**:
- Comprehensive solution covering all cases
- Clear error messages for invalid properties
- Recommended approach

**Cons**:
- More complex implementation
- Requires Unicode property database

**Implementation**:
```rust
// Add Unicode property parser
struct UnicodePropertyParser {
    property_database: PropertyDatabase,
}

// Parse Unicode property syntax
fn parse_unicode_property() -> Node {
    // Parse \p{...} or \P{...}
}
```

#### Option 2: Regex-Based Matching

**Pros**:
- Simpler implementation
- Faster for simple cases

**Cons**:
- May not handle all edge cases
- Limited regex capabilities for complex properties

**Implementation**:
```rust
// Use regex to match Unicode property syntax
let property_regex = Regex::new(r"\\p\{([^}]*)\}")?;
```

#### Option 3: Conservative Fallback

**Pros**:
- Simplest implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all Unicode properties
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to literal string when Unicode property detected
fn parse_unicode_property() -> Node {
    // Parse as literal string
    parse_literal()
}
```

### Path Forward

**Recommended**: Option 1 (Unicode Property Parser)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's Unicode support

**Implementation Steps**:
1. Implement Unicode property parser with property database
2. Add Unicode property parsing logic to lexer/parser
3. Handle nested properties correctly
4. Add test cases for Unicode property patterns
5. Update LSP providers to handle new Unicode property nodes
6. Document Unicode property parsing rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 3 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for Unicode property parsing

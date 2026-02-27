# Issue: Branch Reset Groups

## Problem Description

### What We Found

Perl's regex engine supports branch reset groups:
- `(?|pattern)` - branch reset group
- Resets capture groups for each branch
- Allows multiple capture group branches
- Complex patterns: `(?|(a)(b)|(c)(d))` - resets captures for each branch

This creates a **P2 medium risk** for parser complexity:
- Complex branch reset syntax parsing
- Potential for incorrect capture group handling
- No comprehensive test coverage for branch reset groups
- Edge cases: nested branch reset, mixed capture groups

### Minimal Reproduction

```perl
# Branch reset group regex
my $pattern = qr/(?|(a)(b)|(c)(d))/;
my $match = $text =~ /(?|(a)(b)|(c)(d))/;

# Complex branch reset
my $complex = qr/(?|(a{2})(b{2})|(c{2})(d{2}))/;
```

### Current Behavior

The parser may:
- Not correctly parse branch reset group syntax (`(?|...)`)
- Not handle capture group reset correctly
- Not support nested branch reset groups
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Correctly parse branch reset group syntax (`(?|...)`)
- Handle capture group reset correctly
- Support nested branch reset groups
- Generate correct AST structure
- Provide clear error messages for invalid branch reset groups
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with regex handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for branch reset groups

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly parse branch reset group syntax (`(?|...)`)
   - [ ] Handle capture group reset correctly
   - [ ] Support nested branch reset groups
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for invalid branch reset groups
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple branch reset: `(?|(a)(b)|(c)(d))`
     - [ ] Nested branch reset: `(?|(?|(a)(b))(c)(d))`
     - [ ] Complex branch reset patterns
     - [ ] Mixed capture groups with branch reset
     - [ ] Edge cases: empty branch reset, invalid branch reset
     - [ ] Error recovery: invalid branch reset syntax
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for invalid branch reset groups
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for branch reset groups
   - [ ] Code completion suggests valid branch reset patterns

4. **Documentation**:
   - [ ] Branch reset group parsing rules documented
   - [ ] Examples of branch reset patterns
   - [ ] API documentation updated with branch reset handling

### Solution Options

#### Option 1: Branch Reset Parser (Recommended)

**Pros**:
- Comprehensive solution covering all cases
- Clear error messages for invalid branch reset groups
- Recommended approach

**Cons**:
- More complex implementation
- Requires capture group state management

**Implementation**:
```rust
// Add branch reset parser
struct BranchResetParser {
    capture_groups: Vec<CaptureGroup>,
}

// Parse branch reset syntax
fn parse_branch_reset() -> Node {
    // Parse (?|...)
}
```

#### Option 2: Regex-Based Matching

**Pros**:
- Simpler implementation
- Faster for simple cases

**Cons**:
- May not handle all edge cases
- Limited regex capabilities for complex branch reset groups

**Implementation**:
```rust
// Use regex to match branch reset syntax
let branch_reset_regex = Regex::new(r"\(\?\|([^)]*)\)")?;
```

#### Option 3: Conservative Fallback

**Pros**:
- Simplest implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all branch reset group patterns
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to literal string when branch reset detected
fn parse_branch_reset() -> Node {
    // Parse as literal string
    parse_literal()
}
```

### Path Forward

**Recommended**: Option 1 (Branch Reset Parser)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's regex engine

**Implementation Steps**:
1. Implement branch reset parser with capture group state management
2. Add branch reset parsing logic to lexer/parser
3. Handle nested branch reset groups correctly
4. Add test cases for branch reset patterns
5. Update LSP providers to handle new branch reset nodes
6. Document branch reset parsing rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 3 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for branch reset group parsing

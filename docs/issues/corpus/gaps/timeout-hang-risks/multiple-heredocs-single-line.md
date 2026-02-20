# Issue: Multiple Heredocs on Single Line

## Problem Description

### What We Found

Perl allows multiple heredocs on a single line:
- `my $a = <<END; my $b = <<END2;` - Multiple heredocs on same line
- Each heredoc has its own terminator
- Potential for parser confusion
- No test coverage for multiple heredocs on single line

This creates a **P1 high risk** for parser complexity:
- Complex heredoc parsing logic
- Potential for incorrect terminator matching
- No comprehensive test coverage for multiple heredocs
- Edge cases: nested heredocs, mixed terminators

### Minimal Reproduction

```perl
# Multiple heredocs on single line
my $a = <<END;
This is heredoc a
END
my $b = <<END2;
This is heredoc b
END2

# Multiple heredocs on single line
my $x = <<X; my $y = <<Y; my $z = <<Z;
Content X
X
Content Y
Y
Content Z
Z
```

### Current Behavior

The parser may:
- Not correctly handle multiple heredocs on single line
- Not match terminators correctly
- Not support multiple heredocs simultaneously
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Correctly handle multiple heredocs on single line
- Match terminators correctly for each heredoc
- Support multiple heredocs simultaneously
- Generate correct AST structure
- Provide clear error messages for unmatched terminators
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with heredoc handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for multiple heredocs

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly handle multiple heredocs on single line
   - [ ] Match terminators correctly for each heredoc
   - [ ] Support multiple heredocs simultaneously
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for unmatched terminators
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple multiple heredocs: `my $a = <<END; my $b = <<END2;`
     - [ ] Multiple heredocs on single line: `my $x = <<X; my $y = <<Y; my $z = <<Z;`
     - [ ] Nested heredocs with multiple levels
     - [ ] Edge cases: empty heredocs, mixed terminators
     - [ ] Error recovery: unmatched terminators
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for heredoc issues
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for heredocs
   - [ ] Code completion suggests appropriate heredoc usage

4. **Documentation**:
   - [ ] Multiple heredoc parsing rules documented
   - [ ] Examples of multiple heredoc scenarios
   - [ ] API documentation updated with heredoc handling

### Solution Options

#### Option 1: Heredoc Stack (Recommended)

**Pros**:
- Handles multiple heredocs correctly
- Clear error messages for unmatched terminators
- Recommended approach

**Cons**:
- More complex implementation
- Requires heredoc state management

**Implementation**:
```rust
// Add heredoc stack to parser
struct HeredocParser {
    heredoc_stack: Vec<HeredocContext>,
}

// Parse heredocs with stack
fn parse_heredocs() -> Node {
    // Parse with stack-based tracking
}
```

#### Option 2: Sequential Parsing

**Pros**:
- Simpler implementation
- Faster for simple cases

**Cons**:
- May not handle all edge cases
- Limited support for complex heredoc scenarios

**Implementation**:
```rust
// Parse heredocs sequentially
fn parse_heredocs() -> Node {
    // Parse heredocs one at a time
    parse_heredoc()
}
```

#### Option 3: Conservative Fallback

**Pros**:
- Simplest implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all multiple heredoc patterns
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to single heredoc when multiple detected
fn parse_heredocs() -> Node {
    // Parse as single heredoc
    parse_heredoc()
}
```

### Path Forward

**Recommended**: Option 1 (Heredoc Stack)

**Rationale**:
- Handles multiple heredocs correctly
- Provides clear error messages
- Recommended approach
- Aligns with Perl's heredoc behavior

**Implementation Steps**:
1. Implement heredoc stack in lexer/parser
2. Add heredoc parsing logic with stack management
3. Handle multiple heredocs on single line
4. Add test cases for multiple heredoc scenarios
5. Update LSP providers to handle new heredoc nodes
6. Document multiple heredoc parsing rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 3-5 days for implementation + 2 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for heredoc parsing

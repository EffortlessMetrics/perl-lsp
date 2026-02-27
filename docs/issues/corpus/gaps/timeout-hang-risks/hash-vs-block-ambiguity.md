# Issue: Hash vs Block Ambiguity

## Problem Description

### What We Found

Perl's `{}` syntax has dual meaning:
1. **Hash reference**: `{ key => value }` - creates a hash reference
2. **Code block**: `{ statement; }` - creates a code block

This creates a **P2 medium risk** for parser ambiguity:
- No clear disambiguation rules for `{}` context
- Parser may incorrectly parse hash as block or vice versa
- No test coverage for ambiguous `{}` scenarios
- Potential for incorrect AST generation

### Minimal Reproduction

```perl
# Ambiguous {} - hash vs block
my $hash = { key => 'value' };  # Hash reference
my $code = { print "hello"; };   # Code block

# Another ambiguous case
sub my_sub {
    my $hash = { key => 'value' };
    my $code = { print "hello"; };
}
```

### Current Behavior

The parser may:
- Parse `{}` as hash in all contexts
- Parse `{}` as block in all contexts
- Not provide context-aware parsing
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Disambiguate `{}` based on context:
  - Hash: `{ key => value }` when used in assignment or expression
  - Block: `{ statement; }` when used in control structures or subroutines
  - Provide clear error messages for ambiguous cases
- Generate correct AST structure
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with brace handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for hash/block ambiguity

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Context-aware `{}` parsing based on preceding/following tokens
   - [ ] Proper disambiguation rules for hash vs block
   - [ ] Clear error messages for ambiguous cases
   - [ ] Correct AST generation for both hash and block
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 6 test cases covering:
     - [ ] Simple hash: `{ key => 'value' }`
     - [ ] Simple block: `{ print "hello"; }`
     - [ ] Hash in assignment: `my $hash = { key => 'value' };`
     - [ ] Block in control structure: `if ($cond) { print "hello"; }`
     - [ ] Nested `{}` with mixed contexts
     - [ ] Edge cases: empty `{}`, whitespace
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for ambiguous `{}` usage
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for both hash and block
   - [ ] Code completion suggests appropriate `{}` usage

4. **Documentation**:
   - [ ] Hash vs block disambiguation rules documented
   - [ ] Examples of ambiguous cases and resolution
   - [ ] API documentation updated with `{}` handling

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
enum BraceContext {
    Hash,
    Block,
    Unknown,
}

// Determine brace context based on preceding tokens
fn determine_brace_context(tokens: &[Token]) -> BraceContext {
    // Look at previous token to determine context
    // If previous token is assignment or expression, likely hash
    // If previous token is control structure or subroutine, likely block
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
// Default to hash when ambiguous
fn parse_braces() -> Node {
    // Always parse as hash unless clear block context
    parse_hash()
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
my $hash = +{ key => 'value' };  // Explicit hash
my $code = sub { print "hello"; };  // Explicit block
```

### Path Forward

**Recommended**: Option 1 (Context-Aware Parsing)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's actual parsing behavior

**Implementation Steps**:
1. Add `BraceContext` enum to track context
2. Implement context determination logic in lexer/parser
3. Update `{}` parsing to use context
4. Add test cases for ambiguous scenarios
5. Update LSP providers to handle new hash/block nodes
6. Document hash vs block disambiguation rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 2 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for hash/block disambiguation

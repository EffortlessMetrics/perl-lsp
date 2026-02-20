# Issue: Recursive Heredoc Terminators Risk

## Problem Description

### What We Found

Heredoc terminators create a **P1 high risk** for parser hangs:
- No protection against recursive heredoc terminators
- No timeout for heredoc parsing
- No test coverage for recursive heredoc scenarios
- Potential for infinite loops during parsing
- No validation of heredoc syntax correctness

This represents a **high risk** that could cause the parser to hang or consume excessive memory.

### Minimal Reproduction

```perl
# Recursive heredoc terminators
my $code = <<'END';
print $code;
print "Before END\n";

my $heredoc = <<'END';
print $heredoc;
print "After END\n";
```

### Current Behavior

The parser may:
- Hang indefinitely parsing nested heredocs
- Enter an infinite loop
- Consume excessive memory
- Provide no error messages
- Crash or timeout

### Expected Behavior

The parser should:
- Detect recursive heredoc terminators
- Limit recursion depth or iteration count
- Provide clear error messages
- Timeout and return with error instead of hanging
- Generate correct AST structure

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with heredoc handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for heredoc scenarios

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Detect recursive heredoc terminators
   - [ ] Implement recursion depth limit
   - [ ] Add timeout protection
   - [ ] Provide clear error messages

2. **Test Coverage**:
   - [ ] At least 4 test cases covering:
     - Simple nested heredocs
     - Multiple levels of nesting
     - Edge cases: empty heredocs, invalid terminators
     - Error recovery scenarios

3. **LSP Integration**:
   - [ ] Correct diagnostics for heredoc issues
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly

4. **Documentation**:
   - [ ] Heredoc parsing rules documented
   - [ ] Examples of recursive cases and resolution
   - [ ] API documentation updated with heredoc handling

### Solution Options

#### Option 1: Recursion Depth Limit (Recommended)

**Pros**:
- Simple and effective
- Clear error messages
- Prevents infinite loops

**Cons**:
- May not handle all edge cases
- May reject valid deep nesting

**Implementation**:
```rust
const MAX_HEREDOC_DEPTH: usize = 100;

// In heredoc parser
fn parse_heredoc() -> Node {
    let mut depth = 0;
    // Parse with depth tracking
}
```

#### Option 2: Timeout Protection

**Pros**:
- Prevents hangs
- Configurable timeout
- Works with other protection mechanisms

**Cons**:
- More complex
- May interrupt valid long heredocs

**Implementation**:
```rust
use std::time::Instant;

const HEREDOC_TIMEOUT: Duration = Duration::from_secs(5);

// In heredoc parser
fn parse_heredoc_with_timeout() -> Result<Node, ParseError> {
    let start = Instant::now();
    let result = parse_heredoc();
    if start.elapsed() > HEREDOC_TIMEOUT {
        return Err(ParseError::Timeout);
    }
    result
}
```

#### Option 3: State Machine Approach

**Pros**:
- Most robust solution
- Handles all edge cases

**Cons**:
- Most complex implementation
- Requires significant parser changes

**Implementation**:
```rust
enum HeredocParserState {
    Start,
    InHeredoc,
    LookingForEnd,
}

fn parse_heredoc_with_state() -> Node {
    let mut state = HeredocParserState::Start;
    // State machine parsing
}
```

### Path Forward

**Recommended**: Option 1 (Recursion Depth Limit)

**Rationale**:
- Simplest solution that addresses the core risk
- Provides clear user feedback
- Can be implemented quickly
- Aligns with parser architecture

**Implementation Steps**:
1. Add `MAX_HEREDOC_DEPTH` constant to parser
2. Implement depth tracking in heredoc parser
3. Add timeout protection in parser wrapper
4. Add test cases for recursive heredocs
5. Update LSP providers to handle new heredoc errors
6. Document heredoc parsing rules
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

# Issue: Source Filter Code Execution

## Problem Description

### What We Found

Perl's source filter mechanism allows code execution:
- `use Filter;` - Source filter pragma
- `FILTER_CODE` - Environment variable for filter code
- Executes code during compilation
- Potential for security vulnerabilities

This creates a **P1 high risk** for parser security:
- No protection against malicious source filters
- No validation of filter code
- No test coverage for source filter scenarios
- Potential for code injection attacks

### Minimal Reproduction

```perl
# Source filter code execution
use Filter;

# Filter code in environment
BEGIN {
    $ENV{FILTER_CODE} = 'print "executed code";';
}

# Source filter with code execution
use Filter 'print "malicious code";';
```

### Current Behavior

The parser may:
- Not detect source filter code execution
- Not validate filter code safety
- Not provide security warnings
- Generate incorrect AST structure
- Execute code without proper sandboxing

### Expected Behavior

The parser should:
- Detect source filter code execution
- Validate filter code safety
- Provide security warnings for dangerous filters
- Generate correct AST structure
- Handle edge cases correctly
- Provide clear error messages for malicious code

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with source filter handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for source filters

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Detect source filter code execution
   - [ ] Validate filter code safety
   - [ ] Provide security warnings for dangerous filters
   - [ ] Generate correct AST structure
   - [ ] Handle edge cases correctly
   - [ ] Provide clear error messages for malicious code

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple source filter: `use Filter;`
     - [ ] Filter code in environment: `$ENV{FILTER_CODE}`
     - [ ] Malicious filter code scenarios
     - [ ] Edge cases: empty filters, invalid filters
     - [ ] Error recovery: malicious filter code
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct security diagnostics for source filters
   - [ ] Hover provides security context
   - [ ] Go-to-definition works correctly for source filters
   - [ ] Code completion warns about dangerous filters

4. **Documentation**:
   - [ ] Source filter security rules documented
   - [ ] Examples of safe and unsafe filters
   - [ ] API documentation updated with source filter handling

### Solution Options

#### Option 1: Source Filter Security Analyzer (Recommended)

**Pros**:
- Comprehensive security analysis
- Clear security warnings
- Recommended approach

**Cons**:
- More complex implementation
- Requires security expertise

**Implementation**:
```rust
// Add source filter security analyzer
struct SourceFilterAnalyzer {
    // Security analysis state
}

// Analyze source filter code
fn analyze_source_filter() -> SecurityResult {
    // Analyze filter code for security issues
}
```

#### Option 2: Conservative Blocking

**Pros**:
- Simplest implementation
- Maximum security

**Cons**:
- May break legitimate use cases
- Could be too restrictive

**Implementation**:
```rust
// Block all source filters
fn parse_source_filter() -> Result<Node, ParseError> {
    // Always return error for source filters
    Err(ParseError::SourceFilterBlocked)
}
```

#### Option 3: Warning-Based Approach

**Pros**:
- Allows legitimate use cases
- Provides security warnings
- Less restrictive than blocking

**Cons**:
- May not catch all security issues
- Relies on user attention to warnings

**Implementation**:
```rust
// Warn about source filters
fn parse_source_filter() -> Node {
    // Parse with security warnings
    warn_about_source_filter()
}
```

### Path Forward

**Recommended**: Option 1 (Source Filter Security Analyzer)

**Rationale**:
- Most comprehensive security solution
- Provides clear security warnings
- Recommended approach
- Balances security and usability

**Implementation Steps**:
1. Implement source filter security analyzer
2. Add security analysis logic to lexer/parser
3. Provide security warnings for dangerous filters
4. Add test cases for source filter scenarios
5. Update LSP providers to handle security diagnostics
6. Document source filter security rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 3 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Security Guide**: [Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for source filter security

# Issue: Regex Code Execution

## Problem Description

### What We Found

Perl's regex engine supports code execution:
- `(?{...})` - Embedded code in regex
- Executes code during pattern matching
- Potential for security vulnerabilities
- No protection against malicious regex code

This creates a **P1 high risk** for parser security:
- No protection against malicious regex code execution
- No validation of regex code safety
- No test coverage for regex code execution
- Potential for code injection attacks

### Minimal Reproduction

```perl
# Regex code execution
my $pattern = qr/(?{print "executed code"})/;
my $match = $text =~ /(?{print "executed code"})/;

# Malicious regex code
my $malicious = qr/(?{system "rm -rf /"})/;
```

### Current Behavior

The parser may:
- Not detect regex code execution
- Not validate regex code safety
- Not provide security warnings
- Generate incorrect AST structure
- Execute code without proper sandboxing

### Expected Behavior

The parser should:
- Detect regex code execution patterns
- Validate regex code safety
- Provide security warnings for dangerous regex
- Generate correct AST structure
- Handle edge cases correctly
- Provide clear error messages for malicious regex

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with regex handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for regex code execution

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Detect regex code execution patterns
   - [ ] Validate regex code safety
   - [ ] Provide security warnings for dangerous regex
   - [ ] Generate correct AST structure
   - [ ] Handle edge cases correctly
   - [ ] Provide clear error messages for malicious regex

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple regex code execution: `(?{print "hello"})`
     - [ ] Malicious regex code: `(?{system "rm -rf /"})`
     - [ ] Edge cases: empty regex code, invalid regex code
     - [ ] Error recovery: malicious regex code
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct security diagnostics for regex code execution
   - [ ] Hover provides security context
   - [ ] Go-to-definition works correctly for regex patterns
   - [ ] Code completion warns about dangerous regex patterns

4. **Documentation**:
   - [ ] Regex code execution security rules documented
   - [ ] Examples of safe and unsafe regex patterns
   - [ ] API documentation updated with regex security handling

### Solution Options

#### Option 1: Regex Code Security Analyzer (Recommended)

**Pros**:
- Comprehensive security analysis
- Clear security warnings
- Recommended approach

**Cons**:
- More complex implementation
- Requires security expertise

**Implementation**:
```rust
// Add regex code security analyzer
struct RegexCodeAnalyzer {
    // Security analysis state
}

// Analyze regex code for security issues
fn analyze_regex_code() -> SecurityResult {
    // Analyze regex code for security issues
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
// Block all regex code execution
fn parse_regex_code() -> Result<Node, ParseError> {
    // Always return error for regex code execution
    Err(ParseError::RegexCodeBlocked)
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
// Warn about regex code execution
fn parse_regex_code() -> Node {
    // Parse with security warnings
    warn_about_regex_code()
}
```

### Path Forward

**Recommended**: Option 1 (Regex Code Security Analyzer)

**Rationale**:
- Most comprehensive security solution
- Provides clear security warnings
- Recommended approach
- Balances security and usability

**Implementation Steps**:
1. Implement regex code security analyzer
2. Add security analysis logic to lexer/parser
3. Provide security warnings for dangerous regex
4. Add test cases for regex code execution scenarios
5. Update LSP providers to handle security diagnostics
6. Document regex code execution security rules
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
- **GA Feature Alignment**: GA feature for regex security

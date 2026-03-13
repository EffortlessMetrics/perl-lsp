# Issue: Catastrophic Regex Backtracking Risk

## Problem Statement

Complex regex patterns pose a **P0 critical risk** for catastrophic backtracking, a class of algorithmic complexity attacks where certain regex patterns exhibit exponential time complexity on specific inputs. This can cause:

1. **Parser hangs**: Regex parsing may never complete
2. **Denial of service**: LSP server becomes unresponsive
3. **Excessive resource usage**: CPU and memory exhaustion

### Why This Causes Timeout/Hang Risk

Catastrophic backtracking occurs when the regex engine must explore an exponential number of possible matches. This happens with:

- **Nested quantifiers**: `(a+)+` causes exponential backtracking
- **Overlapping alternatives**: `(a|a)+` creates ambiguous paths
- **Back-references with quantifiers**: `(.)\1+` on repeated characters

The time complexity can be O(2^n) where n is the input length, making even modest inputs (100 characters) cause millions of backtracking steps.

### Exponential Time Complexity Explained

```
Pattern: (a+)+b
Input:   aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac

Step 1: Outer + tries to match all 'a's
Step 2: Inner + tries different distributions
Step 3: On failure, backtrack and retry
Step 4: Exponential combinations explored

For n=30 characters: ~1 billion backtracking steps
For n=40 characters: ~1 trillion backtracking steps
```

## Impact Assessment

| Aspect | Details |
|--------|---------|
| **Severity** | P0 Critical |
| **Category** | Security / Stability / Performance |
| **Affected Features** | Regex Parsing, LSP Server, Editor Integration |
| **User Impact** | Editor freeze, CPU spike, potential crash |
| **Attack Vector** | Perl files with malicious regex patterns |

### Affected Components

- **Lexer**: Regex literal tokenization
- **Parser**: Pattern parsing in match/replace operations
- **LSP Server**: Could hang when analyzing files
- **Editor**: VSCode may freeze when opening malicious files

### Real-World Impact

| Pattern Type | Input Size | Time to Hang |
|--------------|------------|--------------|
| `(a+)+b` on "aaa...aac" | 30 chars | ~1 second |
| `(a+)+b` on "aaa...aac" | 40 chars | ~10 seconds |
| `(a+)+b` on "aaa...aac" | 50 chars | ~100 seconds |
| `(a|aa|aaa)+` on "aaa..." | 30 chars | Exponential |

### Attack Classification

This is a form of **ReDoS (Regular Expression Denial of Service)**, classified as:

- **CWE-1333**: Inefficient Regular Expression Complexity
- **OWASP Category**: Denial of Service

## Technical Details

### Root Cause

Regex backtracking occurs when the engine tries different ways to match a pattern:

1. **Greedy quantifiers** try to match as much as possible
2. On failure, engine **backtracks** to try shorter matches
3. **Nested quantifiers** multiply the backtracking paths
4. Result: **Exponential** number of attempts

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Regex Byte Limit | [`crates/perl-lexer/src/lib.rs:172`](../../../../../../crates/perl-lexer/src/lib.rs) | `MAX_REGEX_BYTES = 64KB` |
| Heredoc Byte Limit | [`crates/perl-lexer/src/lib.rs:173`](../../../../../../crates/perl-lexer/src/lib.rs) | `MAX_HEREDOC_BYTES = 256KB` |
| Delimiter Nesting | [`crates/perl-lexer/src/lib.rs:174`](../../../../../../crates/perl-lexer/src/lib.rs) | `MAX_DELIM_NEST = 128` |
| Heredoc Depth | [`crates/perl-lexer/src/lib.rs:175`](../../../../../../crates/perl-lexer/src/lib.rs) | `MAX_HEREDOC_DEPTH = 100` |
| Lexer Implementation | [`crates/perl-lexer/src/lib.rs`](../../../../../../crates/perl-lexer/src/lib.rs) | Regex tokenization |

### Current Implementation

From [`crates/perl-lexer/src/lib.rs:171-175`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
// Limits to prevent timeout/hang on pathological input
const MAX_REGEX_BYTES: usize = 64 * 1024;  // 64KB max for regex patterns
const MAX_HEREDOC_BYTES: usize = 256 * 1024; // 256KB max for heredoc bodies
const MAX_DELIM_NEST: usize = 128;         // Max nesting depth for delimiters
const MAX_HEREDOC_DEPTH: usize = 100;      // Max nesting depth for heredocs
```

### Budget Guard Implementation

From [`crates/perl-lexer/src/lib.rs:589-609`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
/// **Limits**:
/// - `MAX_REGEX_BYTES` (64KB): Maximum bytes in a single regex literal
/// - `MAX_DELIM_NEST` (128): Maximum delimiter nesting depth
fn try_regex_with_budget(&mut self) -> Option<Token> {
    let start = self.position;
    // ... parsing logic
    
    let bytes_consumed = self.position - start;
    if bytes_consumed <= MAX_REGEX_BYTES && depth <= MAX_DELIM_NEST {
        return None; // Within budget
    }
    
    // Budget exceeded - emit UnknownRest for graceful degradation
}
```

## Examples

### Classic Catastrophic Backtracking

```perl
# The infamous (a+)+b pattern
my $string = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";

if ($string =~ /^(a+)+b$/) {
    print "Matched!\n";
}
# This will hang for seconds to minutes depending on string length
```

### Nested Quantifiers

```perl
# Multiple nested quantifiers
my $text = "a" x 30;
if ($text =~ /^(a+)+$/) { }      # Exponential
if ($text =~ /^(a*)*$/) { }      # Exponential
if ($text =~ /^(a|a+)+$/) { }    # Exponential
```

### Overlapping Alternatives

```perl
# Alternatives that can match the same content
my $text = "aaaaaaaaaaaaaaaaaaaa";
if ($text =~ /^(a|aa|aaa)+$/) { }  # Exponential paths
```

### Back-reference Attacks

```perl
# Back-references with quantifiers
my $text = "a" x 25 . "b" x 25;
if ($text =~ /^(a+)\1+b$/) { }    # Can be exponential
```

### Safe Alternatives

```perl
# Use atomic grouping (if available) or possessive quantifiers
if ($string =~ /^(?>a+)+b$/) { }  # Atomic grouping prevents backtracking

# Or rewrite to avoid nested quantifiers
if ($string =~ /^a+b$/) { }       # Simple, linear time
```

### Pathological Patterns to Detect

| Pattern | Risk Level | Time Complexity |
|---------|------------|-----------------|
| `(a+)+` | Critical | O(2^n) |
| `(a*)*` | Critical | O(2^n) |
| `(a|aa)+` | High | O(2^n) |
| `(a?){n}` | High | O(2^n) |
| `(.*)\1` | Medium | O(n^2) |

## Current Mitigation

### Implemented Protections

| Protection | Value | Purpose |
|------------|-------|---------|
| `MAX_REGEX_BYTES` | 64KB | Limits regex pattern size |
| `MAX_DELIM_NEST` | 128 | Limits delimiter nesting depth |
| `MAX_HEREDOC_DEPTH` | 100 | Limits heredoc nesting |
| `MAX_HEREDOC_BYTES` | 256KB | Limits heredoc body size |

### How Protections Work

1. **Byte Limit**: Regex patterns exceeding 64KB are truncated or rejected
2. **Nesting Limit**: Delimiter nesting beyond 128 levels fails
3. **Graceful Degradation**: Emits `UnknownRest` token instead of hanging

### Lexer Implementation

From [`crates/perl-lexer/src/lib.rs:2907-2909`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
/// - Budget guard prevents infinite loops on pathological input
/// - MAX_REGEX_BYTES limit (64KB) ensures bounded execution time
/// - Graceful degradation: emit UnknownRest token if budget exceeded
```

### Limitations

1. **No regex execution timeout**: Lexer limits don't prevent runtime backtracking
2. **Pattern analysis limited**: Can't detect all pathological patterns
3. **False positives**: Some valid patterns may hit limits

### Test Coverage

| Test File | Purpose |
|-----------|---------|
| [`lexer_catastrophic_regex_test.rs`](../../../../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs) | Catastrophic backtracking tests |
| [`hang_risk_regex_literal_tests.rs`](../../../../../../crates/perl-lexer/tests/hang_risk_regex_literal_tests.rs) | Regex literal hang risks |

From [`lexer_catastrophic_regex_test.rs`](../../../../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs):

```rust
//! Tests for Issue #424: Fix catastrophic regex backtracking timeout risk
//!
//! This test suite validates that the lexer handles potentially catastrophic regex
//! patterns without timeout or exponential parsing time. Patterns like `(a+)+b` can
//! cause exponential backtracking in some regex engines, but our lexer should handle
//! them safely by limiting iteration count and pattern complexity.
//!
//! - Normal regex patterns: <1ms tokenization time
//! - Pathological patterns: <100ms with budget guard triggering
```

## Proposed Solutions

### Option 1: Regex Pattern Analysis (Recommended)

**Approach**: Analyze regex patterns for backtracking risk before parsing

**Pros**:
- Prevents problematic patterns early
- Can warn users about risky patterns
- No runtime overhead for safe patterns

**Cons**:
- Complex pattern analysis
- May have false positives
- Doesn't cover all cases

**Implementation**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegexRisk {
    Low,      // Safe pattern
    Medium,   // Some risk, warn user
    High,     // Dangerous, reject or truncate
}

fn analyze_regex_risk(pattern: &str) -> RegexRisk {
    // Detect nested quantifiers: (a+)+, (a*)*, etc.
    if NESTED_QUANTIFIER_REGEX.is_match(pattern) {
        return RegexRisk::High;
    }
    
    // Detect overlapping alternatives: (a|aa|aaa)+
    if has_overlapping_alternatives(pattern) {
        return RegexRisk::Medium;
    }
    
    RegexRisk::Low
}

// Pattern to detect nested quantifiers
static NESTED_QUANTIFIER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\([^)]*[+*][^)]*\)[+*]").unwrap()
});
```

### Option 2: Timeout-Based Protection

**Approach**: Add timeout to regex parsing operations

**Pros**:
- Simple to implement
- Catches all hang scenarios
- Already partially implemented for heredocs

**Cons**:
- May timeout on valid slow patterns
- Requires careful tuning
- Platform-dependent timing

**Implementation**:
```rust
const REGEX_PARSE_TIMEOUT_MS: u64 = 1000; // 1 second

fn parse_regex_with_timeout(&mut self) -> ParseResult<Token> {
    let start = Instant::now();
    // ... parsing logic
    if start.elapsed().as_millis() > REGEX_PARSE_TIMEOUT_MS {
        return Err(ParseError::Timeout {
            operation: "regex parsing".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        });
    }
}
```

### Option 3: Backtracking Limit

**Approach**: Limit the number of backtracking steps

**Pros**:
- Precise control over complexity
- Matches Perl's built-in protection
- Deterministic behavior

**Cons**:
- Requires regex engine support
- May need configuration
- Not applicable to lexer (only runtime)

**Implementation** (Perl side):
```perl
# In Perl, you can set backtracking limit
use re 'eval';
$regex->backtracking_limit(10000);
```

### Option 4: Warn on Risky Patterns

**Approach**: Emit LSP diagnostics for potentially dangerous patterns

**Pros**:
- Non-blocking
- Educational for users
- Can be disabled

**Cons**:
- False positives may annoy users
- Requires pattern analysis
- Doesn't prevent hangs

**Implementation**:
```rust
fn check_regex_pattern(&self, pattern: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    if has_nested_quantifiers(pattern) {
        diagnostics.push(Diagnostic {
            message: "Regex pattern may cause catastrophic backtracking".to_string(),
            severity: DiagnosticSeverity::Warning,
            code: Some("redundant-nested-quantifier".to_string()),
            source: Some("perl-lsp".to_string()),
            ..Default::default()
        });
    }
    
    diagnostics
}
```

## Testing

### Existing Test Cases

From [`lexer_catastrophic_regex_test.rs`](../../../../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs):

```rust
#[test]
fn test_deeply_nested_delimiters_budget_guard() {
    // Create a pattern with deeply nested delimiters beyond MAX_DELIM_NEST (128)
    let mut pattern = String::from("s{");
    for _ in 0..150 {
        pattern.push_str("{");
    }
    // Should fail gracefully, not hang
}

#[test]
fn test_pathological_patterns_complete_quickly() {
    let pathological = vec![
        (r"/(a+)+$/", "Nested quantifiers"),
        (r"/(a*)*$/", "Nested star quantifiers"),
        (r"/(a|aa|aaa)+$/", "Overlapping alternatives"),
    ];
    
    for (pattern, desc) in pathological {
        let start = Instant::now();
        let mut lexer = PerlLexer::new(pattern);
        let tokens: Vec<_> = lexer.collect();
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(100),
            "{}: should complete in <100ms, took {:?}", desc, elapsed);
    }
}
```

### Required Test Coverage

- [x] Byte limit enforcement (64KB)
- [x] Delimiter nesting limit (128)
- [x] Heredoc depth limit (100)
- [ ] Pattern analysis for nested quantifiers
- [ ] Timeout on pathological patterns
- [ ] Performance: rejection within 100ms
- [ ] Memory bounded during parsing

### Test Patterns to Cover

```perl
# These patterns should be detected/handled
^(a+)+$        # Nested quantifiers
^(a*)*$        # Nested quantifiers
^(a|aa|aaa)+$  # Overlapping alternatives
^(.)\1+$       # Back-reference with quantifier
^(.?){25}$     # Exponential paths
```

### Performance Benchmarks

| Pattern | Input Size | Expected Time |
|---------|------------|---------------|
| Simple `/abc/` | Any | <1ms |
| Complex `/[a-z]+/` | 1KB | <10ms |
| Nested `/(a+)+/` | 100 chars | <100ms (budget guard) |
| Deep nesting `/(a{1}){128}/` | Any | <100ms (limit hit) |

## Related Issues

- Issue #424: Fix catastrophic regex backtracking timeout risk
- Issue #443: Heredoc timeout protection
- Related to overall parser hardening efforts

## References

### Internal Documentation

- [Crate Architecture Guide](../../../../reference/CRATE_ARCHITECTURE_GUIDE.md)
- [Error Handling Strategy ADR](../../../../adr/0012-error-handling-strategy.md)

### Source Code

- [`crates/perl-lexer/src/lib.rs`](../../../../../../crates/perl-lexer/src/lib.rs) - Lexer implementation
- [`crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs`](../../../../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs) - Test coverage
- [`crates/perl-lexer/tests/hang_risk_regex_literal_tests.rs`](../../../../../../crates/perl-lexer/tests/hang_risk_regex_literal_tests.rs) - Hang risk tests

### External References

- [Runaway Regular Expressions: Catastrophic Backtracking](https://www.regular-expressions.info/catastrophic.html)
- [OWASP ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS)
- [Perl re pragma](https://perldoc.perl.org/re)
- [CWE-1333: Inefficient Regular Expression Complexity](https://cwe.mitre.org/data/definitions/1333.html)

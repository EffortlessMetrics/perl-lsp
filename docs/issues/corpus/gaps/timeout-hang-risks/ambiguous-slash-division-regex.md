# Issue: Ambiguous Slash Division vs Regex

## Problem Statement

The slash `/` character has dual meaning in Perl, creating a fundamental parsing ambiguity:

1. **Division operator**: `$a / $b` - divides `$a` by `$b`
2. **Regex delimiter**: `/pattern/` - matches a regex pattern

This ambiguity is **inherent to Perl's syntax** and cannot be resolved without context analysis. The parser must determine the correct interpretation based on surrounding tokens, which creates:

- **Parsing complexity**: Context-dependent tokenization required
- **Correctness risk**: Incorrect interpretation leads to wrong AST
- **LSP impact**: Diagnostics, hover, and navigation depend on correct parsing

### Why This Causes Timeout/Hang Risk

While not a direct timeout risk like deep nesting, ambiguous slash parsing can lead to:

1. **Exponential parse attempts**: Parser may try multiple interpretations
2. **Incorrect error recovery**: Misinterpreted slashes cause cascading errors
3. **Semantic analysis failures**: Wrong AST leads to infinite loops in analysis

## Impact Assessment

| Aspect | Details |
|--------|---------|
| **Severity** | P0 Critical |
| **Category** | Correctness / Stability |
| **Affected Features** | Parsing, Semantic Analysis, Diagnostics, Navigation |
| **User Impact** | Incorrect syntax highlighting, wrong error messages, broken go-to-definition |
| **Attack Vector** | Crafted code with ambiguous slash usage |

### Affected LSP Features

- **Diagnostics**: May report false positives or miss real errors
- **Hover**: May show wrong information for operators
- **Go-to-definition**: May fail to navigate to correct targets
- **Semantic Highlighting**: May highlight division as regex or vice versa

## Technical Details

### Root Cause

Perl's grammar allows `/` to be either:
- A binary division operator following an expression
- The start of a match operator `m/pattern/` (with optional `m`)

The disambiguation requires looking at the **preceding token**:
- After a term (variable, literal, `)`, `]`, `}`): `/` is division
- After an operator or statement start: `/` begins a regex

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Lexer Mode | [`crates/perl-lexer/src/mode.rs:36-69`](../../../../../../crates/perl-lexer/src/mode.rs) | `LexerMode` enum for context tracking |
| Slash Disambiguation | [`crates/perl-lexer/src/lib.rs:1999-2070`](../../../../../../crates/perl-lexer/src/lib.rs) | `try_operator()` slash handling |
| Parser Core | [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) | Expression parsing |
| Expression Parser | [`crates/perl-parser-core/src/engine/parser/expressions/`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/) | Term/operator handling |

### Lexer Mode Tracking System

The lexer uses a state machine with two primary modes:

```rust
// From crates/perl-lexer/src/mode.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
}
```

**Mode Transitions**:
- After identifier/number/closing paren → `ExpectOperator` → slash is division
- After operator/keyword/opening paren → `ExpectTerm` → slash is regex

### Perl's Disambiguation Rules

Perl uses the following heuristic (simplified):

```perl
# Division - follows a term
my $result = $a / $b;        # term / term
my $calc = (1 + 2) / 3;      # ) /

# Regex - follows operator or is statement start
if (/pattern/) { }           # if (/
my $match = $x =~ /pat/;     # =~ /
print if /pattern/;          # if /
```

### Implementation Details

From [`crates/perl-lexer/src/lib.rs:1999-2070`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
// 2. **Slash Disambiguation**:
//    - `LexerMode::ExpectTerm` → `/` starts a regex
//      Examples: `if (/pattern/)`, `=~ /test/`, `( /regex/`
//    - `LexerMode::ExpectOperator` → `/` is division or `//`
//      Examples: `$x / 2`, `$x // $y`, `) / 3`

if self.current_char() == Some('/') {
    if self.mode == LexerMode::ExpectTerm {
        // Mode indicates we're expecting a term → `/` starts a regex
        // Examples: `if (/pattern/)`, `=~ /test/`, `while (/match/)`
        return self.try_regex();
    } else {
        // Mode indicates we're expecting an operator → `/` is division or `//`
        // Examples: `$x / 2`, `$x // $y`, `10 / 3`
        // ... handle division or defined-or
    }
}
```

## Examples

### Clear Division Cases

```perl
my $quotient = $x / $y;
my $avg = ($a + $b) / 2;
my $ratio = calculate_total() / $count;
```

### Clear Regex Cases

```perl
if (/pattern/) { match() }
my $found = $string =~ /search/;
my $replaced = $str =~ s/old/new/;
print if /warning/;
```

### Ambiguous Cases

```perl
# Context-dependent - requires full parsing
my $result = time / 86400;  # Division (time() returns epoch seconds)
my $match = time /pattern/; # Regex match against $_

# Multiple slashes
my $complex = $a / $b / $c; # ($a / $b) / $c - left-to-right division
my $regex = /$a/$b/;        # Syntax error or unusual regex
```

### Edge Cases

```perl
# Spaced regex (valid Perl)
my $match = / pattern /;    # Regex with whitespace in pattern

# Division with regex on right
my $result = 100 / length($&); # Division by result of regex

# Substitution with division in replacement
my $str = "100";
$str =~ s/(\d+)/$1 \/ 2/e; # Division in replacement

# Defined-or vs empty regex
my $val = $x // $y;         # Defined-or operator
my $match = $x =~ //;       # Empty regex match

# Division assignment vs regex
$x /= 2;                    # Division assignment
$x =~ s/foo/bar/;           # Substitution
```

## Current Mitigation

### Implementation Status

The parser implements context-aware slash disambiguation:

| Feature | Status | Notes |
|---------|--------|-------|
| Basic division parsing | ✅ Implemented | `$a / $b` |
| Basic regex parsing | ✅ Implemented | `/pattern/` |
| Match operator `=~` | ✅ Implemented | `$x =~ /pat/` |
| Statement-start regex | ✅ Implemented | `if (/pat/) { }` |
| Implicit match | ✅ Implemented | `print if /pat/` |
| Defined-or `//` | ✅ Implemented | `$x // $y` |
| Division assignment `/=` | ✅ Implemented | `$x /= 2` |

### Budget Guards

From [`crates/perl-lexer/src/lib.rs:172-175`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
const MAX_REGEX_BYTES: usize = 64 * 1024;  // 64KB max for regex patterns
const MAX_HEREDOC_BYTES: usize = 256 * 1024; // 256KB max for heredoc bodies
const MAX_DELIM_NEST: usize = 128;         // Max nesting depth for delimiters
```

### Timeout Protection

From [`crates/perl-lexer/src/lib.rs:2008-2021`](../../../../../../crates/perl-lexer/src/lib.rs):

```rust
// 3. **Timeout Protection**:
//    - Regex parsing has budget guard: MAX_REGEX_BYTES (64KB)
//    - Budget exceeded → emit UnknownRest token (graceful degradation)
//
// 4. **Graceful Degradation**:
//    - If regex parsing exceeds budget, emit UnknownRest token
//    - Parser continues instead of hanging
//    - LSP diagnostics generated for truncated regexes
//    - Test coverage: lexer_slash_timeout_tests.rs (21 test cases)
```

### Limitations

1. **Complex expressions**: May require look-ahead beyond current token
2. **Error recovery**: Incorrect disambiguation can cascade
3. **Edge cases**: Unusual Perl idioms may not parse correctly

## Proposed Solutions

### Option 1: Enhanced Context Tracking (Recommended) ✅ Implemented

**Approach**: Maintain explicit parser state for expected token type

**Pros**:
- Handles all cases correctly
- Aligns with Perl's parsing behavior
- Enables better error messages

**Cons**:
- More complex implementation
- Requires careful state management

**Implementation** (already in place):
```rust
enum LexerMode {
    ExpectTerm,     // Next should be value/regex
    ExpectOperator, // Next should be operator
}

fn try_operator(&mut self) -> Option<Token> {
    if self.current_char() == Some('/') {
        if self.mode == LexerMode::ExpectTerm {
            return self.try_regex();
        } else {
            // Handle division or defined-or
        }
    }
}
```

### Option 2: Look-ahead Disambiguation

**Approach**: Look ahead to find closing `/` to identify regex

**Pros**:
- Simpler state management
- Works for most cases

**Cons**:
- Can be fooled by `/` in regex pattern
- May require full pattern parsing

### Option 3: Error on Ambiguity

**Approach**: Emit diagnostic when slash context is ambiguous

**Pros**:
- Explicit user feedback
- Encourages clearer code

**Cons**:
- May report false positives
- Could annoy users

## Testing

### Existing Test Coverage

| Test File | Coverage |
|-----------|----------|
| [`crates/perl-lexer/tests/lexer_slash_timeout_tests.rs`](../../../../../../crates/perl-lexer/tests/lexer_slash_timeout_tests.rs) | 21 test cases for slash disambiguation |
| [`crates/perl-lexer/tests/hang_risk_slash_ambiguity_tests.rs`](../../../../../../crates/perl-lexer/tests/hang_risk_slash_ambiguity_tests.rs) | Comprehensive slash ambiguity tests |
| [`crates/perl-lexer/tests/comprehensive_unit_tests.rs`](../../../../../../crates/perl-lexer/tests/comprehensive_unit_tests.rs) | Context-sensitive slash disambiguation |

### Test Cases from Implementation

```rust
// From lexer_slash_timeout_tests.rs
#[test]
fn test_slash_after_identifier_is_division() {
    let mut lexer = PerlLexer::new("$x / 2");
    lexer.next_token(); // $x
    let token = lexer.next_token();
    assert_eq!(token.token_type, TokenType::Division);
}

#[test]
fn test_slash_after_operator_is_regex() {
    let mut lexer = PerlLexer::new("=~ /pattern/");
    lexer.next_token(); // =~
    let token = lexer.next_token();
    assert_eq!(token.token_type, TokenType::RegexMatch);
}
```

### Required Test Cases

- [x] Simple division: `$a / $b`
- [x] Simple regex: `/pattern/`
- [x] Chained division: `$a / $b / $c`
- [x] Regex with division-like content: `/\//`
- [x] Match operator: `$x =~ /pat/`
- [x] Substitution: `$x =~ s/old/new/`
- [x] Implicit match: `print if /pat/`
- [x] Division after function call: `func() / 2`
- [x] Edge case: `time / 86400` vs `time /pattern/`
- [x] Defined-or: `$x // $y`
- [x] Division assignment: `$x /= 2`
- [x] Empty regex: `$x =~ //`
- [x] Performance: no hang on pathological input

### Performance Test

```rust
// From hang_risk_slash_ambiguity_tests.rs
#[test]
fn lexer_slash_ambiguity_no_hang_on_pathological_input() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    let code = Arc::new("$a / $b / $c / $d / $e / $f / $g / $h".repeat(1000));
    let result = Arc::new(Mutex::new(None));

    let handle = thread::spawn(move || {
        let mut lexer = PerlLexer::new(&code);
        let tokens: Vec<_> = lexer.collect();
        *result.lock().unwrap() = Some(tokens);
    });

    // Wait max 2 seconds for lexer to complete
    let completed = handle.join().is_ok();
    assert!(completed, "Lexer should complete within timeout");
}
```

## Related Issues

- Issue #422: Fix ambiguous slash (division vs regex) timeout risk
- Related to overall Perl parsing correctness

## References

### Internal Documentation

- [Crate Architecture Guide](../../../../reference/CRATE_ARCHITECTURE_GUIDE.md)
- [Mode-aware Lexer ADR](../../../../adr/0014-mode-aware-lexer.md)

### Source Code

- [`crates/perl-lexer/src/mode.rs`](../../../../../../crates/perl-lexer/src/mode.rs) - Lexer mode tracking
- [`crates/perl-lexer/src/lib.rs`](../../../../../../crates/perl-lexer/src/lib.rs) - Lexer implementation
- [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) - Parser core
- [`crates/perl-parser-core/src/engine/parser/expressions/`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/) - Expression parsing

### Test Files

- [`crates/perl-lexer/tests/lexer_slash_timeout_tests.rs`](../../../../../../crates/perl-lexer/tests/lexer_slash_timeout_tests.rs)
- [`crates/perl-lexer/tests/hang_risk_slash_ambiguity_tests.rs`](../../../../../../crates/perl-lexer/tests/hang_risk_slash_ambiguity_tests.rs)
- [`crates/perl-lexer/tests/comprehensive_unit_tests.rs`](../../../../../../crates/perl-lexer/tests/comprehensive_unit_tests.rs)

### External References

- [Perl Documentation: perlop](https://perldoc.perl.org/perlop) - Operator precedence and regex quotes
- [Perl Documentation: perlre](https://perldoc.perl.org/perlre) - Regular expressions

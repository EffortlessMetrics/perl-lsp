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
| Lexer | [`crates/perl-lexer/src/lib.rs`](../../../../../../crates/perl-lexer/src/lib.rs) | Slash tokenization |
| Parser Core | [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) | Expression parsing |
| Expression Parser | [`crates/perl-parser-core/src/engine/parser/expressions/`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/) | Term/operator handling |

### Perl's Disambiguation Rules

Perl uses the following heuristic (simplified):

```perl
# Division - follows a term
my $result = $a / $b;# term / term
my $calc = (1 + 2) / 3;    # ) /

# Regex - follows operator or is statement start
if (/pattern/) { }        # if (/
my $match = $x =~ /pat/;   # =~ /
print if /pattern/;        # if /
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

### Lexer Context Tracking

The lexer tracks parsing state to determine slash context:

```rust
// From perl-lexer/src/lib.rs
// The lexer uses state machine to track whether we expect:
// - A term (variable, literal, regex)
// - An operator (including division)
```

### Limitations

1. **Complex expressions**: May require look-ahead beyond current token
2. **Error recovery**: Incorrect disambiguation can cascade
3. **Edge cases**: Unusual Perl idioms may not parse correctly

## Proposed Solutions

### Option 1: Enhanced Context Tracking (Recommended)

**Approach**: Maintain explicit parser state for expected token type

**Pros**:
- Handles all cases correctly
- Aligns with Perl's parsing behavior
- Enables better error messages

**Cons**:
- More complex implementation
- Requires careful state management

**Implementation**:
```rust
enum Expecting {
    Term,     // Next should be value/regex
    Operator, // Next should be operator
}

fn parse_slash(&mut self, expecting: Expecting) -> ParseResult<Node> {
    match expecting {
        Expecting::Term => self.parse_regex(),
        Expecting::Operator => self.parse_division(),
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
| `crates/perl-lexer/tests/` | Lexer slash handling |
| `crates/perl-parser/tests/` | Parser expression tests |

### Required Test Cases

- [ ] Simple division: `$a / $b`
- [ ] Simple regex: `/pattern/`
- [ ] Chained division: `$a / $b / $c`
- [ ] Regex with division-like content: `/\//`
- [ ] Match operator: `$x =~ /pat/`
- [ ] Substitution: `$x =~ s/old/new/`
- [ ] Implicit match: `print if /pat/`
- [ ] Division after function call: `func() / 2`
- [ ] Edge case: `time / 86400` vs `time /pattern/`

## Related Issues

- No open GitHub issues for this specific problem
- Related to overall Perl parsing correctness

## References

### Internal Documentation

- [Crate Architecture Guide](../../../../reference/CRATE_ARCHITECTURE_GUIDE.md)
- [Parser Comparison](../../../../reference/PARSER_COMPARISON.md)

### Source Code

- [`crates/perl-lexer/src/lib.rs`](../../../../../../crates/perl-lexer/src/lib.rs) - Lexer implementation
- [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) - Parser core
- [`crates/perl-parser-core/src/engine/parser/expressions/`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/) - Expression parsing

### External References

- [Perl Documentation: perlop](https://perldoc.perl.org/perlop) - Operator precedence and regex quotes
- [Perl Documentation: perlre](https://perldoc.perl.org/perlre) - Regular expressions

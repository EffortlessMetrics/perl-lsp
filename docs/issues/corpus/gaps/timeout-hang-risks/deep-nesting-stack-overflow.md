# Issue: Deep Nesting Stack Overflow Risk

## Problem Statement

Deep nesting constructs pose a **P0 critical risk** for parser stack overflow. When parsing deeply nested code structures (blocks, parentheses, loops, conditionals), the parser's recursive descent approach can exhaust the call stack, causing:

1. **Parser crashes**: Stack overflow terminates the parser process
2. **Denial of service**: LSP server becomes unresponsive
3. **Security vulnerability**: Malicious code can crash the language server

### Why This Causes Timeout/Hang Risk

The parser uses recursive descent, which creates a new stack frame for each nesting level:

```
parse_statement()
  └── parse_block()
        └── parse_statement()
              └── parse_block()
                    └── ... (continues recursively)
```

With sufficient nesting (typically 1000+ levels), the call stack overflows before any limit triggers, causing immediate process termination.

## Impact Assessment

| Aspect | Details |
|--------|---------|
| **Severity** | P0 Critical |
| **Category** | Security / Stability |
| **Affected Features** | Parsing, LSP Server, Editor Integration |
| **User Impact** | Editor crash, lost work, denial of service |
| **Attack Vector** | Malicious Perl files with extreme nesting |

### Affected Components

- **LSP Server**: Could crash when opening malicious files
- **Editor**: VSCode/other editors could lose language server
- **CI/CD**: Automated analysis could hang or crash
- **Batch Processing**: Corpus processing could fail

### Real-World Impact

| Scenario | Nesting Depth | Risk |
|----------|---------------|------|
| Normal Perl code | 5-20 levels | None |
| Complex frameworks | 20-50 levels | None |
| Generated code | 50-100 levels | Low |
| Malicious/minified | 500+ levels | **Critical** |

## Technical Details

### Root Cause

Recursive descent parsers naturally use the call stack to track parsing state. Each nested construct (block, conditional, loop) adds a stack frame. Without explicit depth limits, the parser relies on the OS stack limit (typically 1-8MB), which translates to roughly 1000-8000 stack frames.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Depth Constant | [`parser/mod.rs:106`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs:106) | `MAX_RECURSION_DEPTH = 128` |
| Depth Check | [`parser/helpers.rs:41`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs:41) | `check_recursion()` |
| Guard Pattern | [`parser/helpers.rs:62`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs:62) | `with_recursion_guard()` |
| Error Type | [`perl-error/src/`](../../../../../../crates/perl-error/src/) | `ParseError::NestingTooDeep` |

### Current Implementation

```rust
// From crates/perl-parser-core/src/engine/parser/mod.rs
// Recursion limit is set conservatively to prevent stack overflow
// before the limit triggers. The actual stack usage depends on the
// number of function frames between recursion checks (about 20-30
// for the precedence parsing chain). 128 * 30 = ~3840 frames which
// is safe. Real Perl code rarely exceeds 20-30 nesting levels.
const MAX_RECURSION_DEPTH: usize = 128;
```

```rust
// From crates/perl-parser-core/src/engine/parser/helpers.rs
#[inline(always)]
fn check_recursion(&mut self) -> ParseResult<()> {
    self.recursion_depth += 1;
    // Fast path: avoid expensive comparisons in the common case
    if self.recursion_depth > MAX_RECURSION_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth: self.recursion_depth,
            max_depth: MAX_RECURSION_DEPTH,
        });
    }
    Ok(())
}
```

## Examples

### Deeply Nested Blocks

```perl
# 150+ levels of nesting - will trigger limit
{
    {
        {
            {
                {
                    # ... 150 more levels
                }
            }
        }
    }
}
```

### Deeply Nested Conditionals

```perl
# Each if/else adds nesting depth
if ($a) {
    if ($b) {
        if ($c) {
            if ($d) {
                if ($e) {
                    # ... 150 more levels
                }
            }
        }
    }
}
```

### Deeply Nested Loops

```perl
# Nested loops add up quickly
for my $i (0..10) {
    for my $j (0..10) {
        for my $k (0..10) {
            # ... 50 more nested loops exceeds limit
        }
    }
}
```

### Deeply Nested Expressions

```perl
# Parentheses in expressions
my $result = (((((((((((((((((((((($x))))))))))))))))))));
```

### Malicious Payload Example

```perl
# This would trigger NestingTooDeep error
# Generated: 200 levels of nesting
sub payload {
    my $code = 'my $x = ';
    for (1..200) {
        $code .= '(';
    }
    $code .= '1';
    for (1..200) {
        $code .= ')';
    }
    $code .= ';';
    return $code;
}
# Result: my $x = (((((((...(((((1)))))...))))));
```

## Current Mitigation

### Implemented Protections

| Protection | Value | Location |
|------------|-------|----------|
| `MAX_RECURSION_DEPTH` | 128 | [`parser/mod.rs:106`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs:106) |
| Error type | `NestingTooDeep` | [`perl-error/`](../../../../../../crates/perl-error/) |
| Guard pattern | `with_recursion_guard()` | [`parser/helpers.rs:62`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs:62) |

### How It Works

1. **Depth Tracking**: Parser maintains `recursion_depth` counter
2. **Increment on Entry**: Each nested construct increments depth
3. **Limit Check**: `check_recursion()` fails if depth > 128
4. **Graceful Error**: Returns `ParseError::NestingTooDeep` with details
5. **Automatic Decrement**: `with_recursion_guard()` ensures cleanup

### Error Response

When nesting exceeds the limit:

```rust
ParseError::NestingTooDeep {
    depth: 129,
    max_depth: 128,
}
```

This produces a user-friendly error message:
```
Nesting too deep: 129 levels exceeds maximum of 128
```

### Test Coverage

| Test File | Purpose |
|-----------|---------|
| [`parser_boundary_validation_tests.rs`](../../../../../../crates/perl-parser/tests/parser_boundary_validation_tests.rs) | Tests limit at exactly 128 |
| [`parser_resource_exhaustion_tests.rs`](../../../../../../crates/perl-parser/tests/parser_resource_exhaustion_tests.rs) | Tests behavior above limit |
| [`hang_risk_deep_nesting_tests.rs`](../../../../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs) | Security-focused nesting tests |
| [`parser_depth_limit_test.rs`](../../../../../../crates/perl-parser/tests/parser_depth_limit_test.rs) | Depth limit validation |
| [`parser_hardening_tests.rs`](../../../../../../crates/perl-parser/tests/parser_hardening_tests.rs) | General hardening tests |

### Known Limitations

1. **Function call nesting**: Some parsing paths may not increment depth counter
2. **Expression nesting**: Complex expressions may hit limit before blocks
3. **Error recovery**: Deep nesting errors may cascade

## Proposed Solutions

### Option 1: Comprehensive Nesting Protection (Current Implementation)

**Status**: ✅ Implemented

**Pros**:
- Complete protection against stack overflow
- Graceful degradation for pathological cases
- Clear error messages
- Configurable limits

**Cons**:
- May reject some valid but pathological code
- Requires careful depth tracking

### Option 2: Iterative Parsing

**Status**: 🔬 Research

**Approach**: Rewrite parser to use explicit stacks instead of recursion

**Pros**:
- Eliminates recursion completely
- No stack overflow risk
- Predictable memory usage

**Cons**:
- Major parser rewrite required
- More complex implementation
- May be slower for normal cases

### Option 3: Configurable Limits

**Status**: 📋 Proposed

**Approach**: Allow users to configure `MAX_RECURSION_DEPTH`

**Pros**:
- Flexibility for different use cases
- Can increase for generated code

**Cons**:
- Risk of misconfiguration
- Higher limits may cause stack overflow

**Implementation**:
```rust
pub struct ParserConfig {
    /// Maximum recursion depth (default: 128)
    pub max_recursion_depth: usize,
}

impl Parser {
    pub fn with_config(input: &str, config: ParserConfig) -> Self {
        // Use config.max_recursion_depth instead of constant
    }
}
```

### Option 4: Timeout Protection

**Status**: 📋 Proposed

**Approach**: Add time-based timeout in addition to depth limit

**Pros**:
- Defense in depth
- Catches other hang scenarios

**Cons**:
- More complex
- May have false positives on slow systems

## Testing

### Existing Test Cases

```rust
// From parser_boundary_validation_tests.rs
const MAX_RECURSION_DEPTH: usize = 128;

#[test]
fn test_recursion_depth_boundary() {
    // Test just below the limit
    let below_limit_code = generate_nested_code(MAX_RECURSION_DEPTH - 5);
    let result = parse(&below_limit_code);
    assert!(result.is_ok(), "Should parse below limit");
    
    // Test just above the limit
    let above_limit_code = generate_nested_code(MAX_RECURSION_DEPTH + 5);
    let result = parse(&above_limit_code);
    assert!(result.is_err(), "Should fail above limit");
}
```

### Required Test Coverage

- [x] Below limit parsing succeeds
- [x] At limit parsing succeeds or fails gracefully
- [x] Above limit returns `NestingTooDeep` error
- [x] Error message includes depth information
- [x] Parser recovers after hitting limit
- [ ] Performance: fails within 2 seconds
- [ ] Memory: bounded usage at extreme depths

## Related Issues

- No open GitHub issues for this specific problem
- Related to overall parser hardening efforts

## References

### Internal Documentation

- [Crate Architecture Guide](../../../../reference/CRATE_ARCHITECTURE_GUIDE.md)
- [Error Handling Strategy](../../../../explanation/ERROR_HANDLING_STRATEGY.md)
- [Security Development Guide](../../../../how-to/SECURITY_DEVELOPMENT_GUIDE.md)

### Source Code

- [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) - Depth constant
- [`crates/perl-parser-core/src/engine/parser/helpers.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) - Depth checking
- [`crates/perl-error/src/`](../../../../../../crates/perl-error/src/) - Error types

### Test Files

- [`crates/perl-parser/tests/parser_boundary_validation_tests.rs`](../../../../../../crates/perl-parser/tests/parser_boundary_validation_tests.rs)
- [`crates/perl-parser/tests/parser_resource_exhaustion_tests.rs`](../../../../../../crates/perl-parser/tests/parser_resource_exhaustion_tests.rs)
- [`crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs`](../../../../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs)
- [`crates/perl-parser/tests/parser_depth_limit_test.rs`](../../../../../../crates/perl-parser/tests/parser_depth_limit_test.rs)

### External References

- [RFC 7230 - Security Considerations for Parsers](https://tools.ietf.org/html/rfc7230)
- [OWASP - Denial of Service](https://owasp.org/www-community/attacks/Denial_of_Service)

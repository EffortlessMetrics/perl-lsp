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
| Depth Constant | [`crates/perl-parser-core/src/engine/parser/mod.rs:106`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) | `MAX_RECURSION_DEPTH = 128` |
| Depth Check | [`crates/perl-parser-core/src/engine/parser/helpers.rs:41`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) | `check_recursion()` |
| Exit Recursion | [`crates/perl-parser-core/src/engine/parser/helpers.rs:53`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) | `exit_recursion()` |
| Guard Pattern | [`crates/perl-parser-core/src/engine/parser/helpers.rs:65`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) | `with_recursion_guard()` |
| Error Type | [`crates/perl-error/src/`](../../../../../../crates/perl-error/src/) | `ParseError::NestingTooDeep` |

### Current Implementation

From [`crates/perl-parser-core/src/engine/parser/mod.rs:79-106`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs):

```rust
/// Current recursion depth for overflow protection during complex Perl script parsing
recursion_depth: usize,

// Recursion limit is set conservatively to prevent stack overflow
// before the limit triggers. The actual stack usage depends on the
// number of function frames between recursion checks (about 20-30
// for the precedence parsing chain). 128 * 30 = ~3840 frames which
// is safe. Real Perl code rarely exceeds 20-30 nesting levels.
const MAX_RECURSION_DEPTH: usize = 128;
```

From [`crates/perl-parser-core/src/engine/parser/helpers.rs:40-55`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs):

```rust
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

fn exit_recursion(&mut self) {
    self.recursion_depth = self.recursion_depth.saturating_sub(1);
}
```

### Guard Pattern Implementation

From [`crates/perl-parser-core/src/engine/parser/helpers.rs:58-75`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs):

```rust
/// Execute a closure with recursion guard (RAII pattern)
///
/// - `check_recursion()` increments depth (and may error)
/// - depth is decremented on scope exit (even on early return / panic)
fn with_recursion_guard<T>(
    &mut self,
    f: impl FnOnce(&mut Self) -> ParseResult<T>,
) -> ParseResult<T> {
    self.check_recursion()?;
    // ... execute closure
    // exit_recursion() called on drop
}
```

### Recursion Check Points

The recursion check is called at strategic points in the parser:

| Location | File | Purpose |
|----------|------|---------|
| Hash/Block parsing | [`expressions/hashes.rs:34`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/hashes.rs) | `parse_hash_or_block_inner()` |
| Function call parsing | [`expressions/calls.rs:145`](../../../../../../crates/perl-parser-core/src/engine/parser/expressions/calls.rs) | Indirect call nesting |

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

### Real-World Deep Nesting Scenarios

```perl
# Deeply nested data structures
my $data = {
    level1 => {
        level2 => {
            level3 => {
                # ... continues 100+ levels
            }
        }
    }
};

# Deeply nested subroutine calls
my $result = func1(func2(func3(func4(
    # ... 100+ nested calls
))));

# Deeply nested ternary operators
my $value = $a ? $b : $c ? $d : $e ? $f :
    # ... 100+ ternary conditions
    $final;
```

## Current Mitigation

### Implemented Protections

| Protection | Value | Location |
|------------|-------|----------|
| `MAX_RECURSION_DEPTH` | 128 | [`parser/mod.rs:106`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) |
| Error type | `NestingTooDeep` | [`perl-error/`](../../../../../../crates/perl-error/) |
| Guard pattern | `with_recursion_guard()` | [`parser/helpers.rs:65`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) |

### How It Works

1. **Depth Tracking**: Parser maintains `recursion_depth` counter
2. **Increment on Entry**: Each nested construct increments depth
3. **Limit Check**: `check_recursion()` fails if depth > 128
4. **Graceful Error**: Returns `ParseError::NestingTooDeep` with details
5. **Automatic Decrement**: `with_recursion_guard()` ensures cleanup via RAII

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

### Stack Safety Calculation

The limit of 128 is calculated based on:

```
Typical stack frames between checks: ~20-30 (precedence parsing chain)
Maximum safe depth: 128 * 30 = ~3840 frames
OS stack limit: typically 1-8MB
Frame size: ~1-2KB
Safe frames: ~500-4000

128 was chosen to be well within safety margin while allowing
reasonable code complexity.
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

### Option 1: Comprehensive Nesting Protection (Current Implementation) ✅ Implemented

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

**Implementation Sketch**:
```rust
struct ParserStack {
    frames: Vec<ParseFrame>,
}

enum ParseFrame {
    Block { depth: usize },
    Expression { precedence: u8 },
    Statement { kind: StmtKind },
}

fn parse_iterative(&mut self) -> ParseResult<Node> {
    let mut stack = ParserStack::new();
    // Explicit stack management instead of recursion
}
```

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

**Implementation**:
```rust
pub struct ParserConfig {
    /// Maximum parse time in milliseconds (default: 5000)
    pub timeout_ms: u64,
}

fn parse_with_timeout(&mut self) -> ParseResult<Node> {
    let start = Instant::now();
    // ... check elapsed time periodically
}
```

## Testing

### Existing Test Cases

From [`parser_boundary_validation_tests.rs`](../../../../../../crates/perl-parser/tests/parser_boundary_validation_tests.rs):

```rust
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

fn generate_nested_code(depth: usize) -> String {
    let mut code = String::new();
    for _ in 0..depth {
        code.push('(');
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push(')');
    }
    code
}
```

From [`hang_risk_deep_nesting_tests.rs`](../../../../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs):

```rust
#[test]
fn parser_hang_risk_nested_blocks_exceed_limit() {
    let depth = 300;
    let mut code = String::new();
    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("my $x = 1;");
    for _ in 0..depth {
        code.push_str("} ");
    }
    
    let result = parse(&code);
    assert!(result.is_err(), "Expected RecursionLimit error for {} nested blocks", depth);
}

#[test]
fn parser_hang_risk_no_hang_on_extreme_nesting() {
    use std::time::Duration;
    
    let depth = 1000;
    let code = generate_nested_code(depth);
    
    let start = Instant::now();
    let result = parse(&code);
    let elapsed = start.elapsed();
    
    // Should fail quickly, not hang
    assert!(elapsed < Duration::from_secs(2), "Should fail within 2 seconds");
    assert!(result.is_err(), "Parser should reject extremely deep nesting");
}
```

### Required Test Coverage

- [x] Below limit parsing succeeds
- [x] At limit parsing succeeds or fails gracefully
- [x] Above limit returns `NestingTooDeep` error
- [x] Error message includes depth information
- [x] Parser recovers after hitting limit
- [x] Performance: fails within 2 seconds
- [ ] Memory: bounded usage at extreme depths

### Test Patterns to Cover

```rust
// Nested blocks
{ { { { ... } } } }

// Nested parentheses
((((...))))

// Nested arrays
[[[...]]]

// Nested hashes
{ a => { b => { c => ... } } }

// Mixed nesting
{ ( [ { ( [ ... ] ) } ] ) }

// Control flow nesting
if (1) { if (1) { if (1) { ... } } }

// Loop nesting
for (;;) { for (;;) { for (;;) { ... } } }

// Subroutine nesting
sub { sub { sub { ... } } }
```

## Related Issues

- No open GitHub issues for this specific problem
- Related to overall parser hardening efforts

## References

### Internal Documentation

- [Crate Architecture Guide](../../../../reference/CRATE_ARCHITECTURE_GUIDE.md)
- [Error Handling Strategy ADR](../../../../adr/0012-error-handling-strategy.md)

### Source Code

- [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/mod.rs) - Depth constant
- [`crates/perl-parser-core/src/engine/parser/helpers.rs`](../../../../../../crates/perl-parser-core/src/engine/parser/helpers.rs) - Depth checking
- [`crates/perl-error/src/`](../../../../../../crates/perl-error/src/) - Error types

### Test Files

- [`crates/perl-parser/tests/parser_boundary_validation_tests.rs`](../../../../../../crates/perl-parser/tests/parser_boundary_validation_tests.rs)
- [`crates/perl-parser/tests/parser_resource_exhaustion_tests.rs`](../../../../../../crates/perl-parser/tests/parser_resource_exhaustion_tests.rs)
- [`crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs`](../../../../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs)
- [`crates/perl-parser/tests/parser_depth_limit_test.rs`](../../../../../../crates/perl-parser/tests/parser_depth_limit_test.rs)
- [`crates/perl-parser/tests/parser_hardening_tests.rs`](../../../../../../crates/perl-parser/tests/parser_hardening_tests.rs)

### External References

- [RFC 7230 - Security Considerations for Parsers](https://tools.ietf.org/html/rfc7230)
- [OWASP - Denial of Service](https://owasp.org/www-community/attacks/Denial_of_Service)
- [CWE-674: Uncontrolled Recursion](https://cwe.mitre.org/data/definitions/674.html)

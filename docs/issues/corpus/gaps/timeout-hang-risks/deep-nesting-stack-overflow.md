# Issue: Deep Nesting Stack Overflow Risk

## Problem Description

### What We Found

Deep nesting constructs pose a **P0 critical risk** for parser stack overflow:
- No explicit nesting depth limits in parser
- No timeout protection for deeply nested constructs
- No test coverage for extreme nesting scenarios
- Potential for unbounded recursion in parser

This represents a **critical security and stability risk** that could cause parser crashes or hangs.

### Minimal Reproduction

```perl
# Deeply nested if/else statements
if ($a) {
    if ($b) {
        if ($c) {
            if ($d) {
                if ($e) {
                    # ... continue nesting
                }
            }
        }
    }
}

# Deeply nested blocks
{
    {
        {
            {
                {
                    # ... continue nesting
                }
            }
        }
    }
}

# Deeply nested loops
for my $i (0..10) {
    for my $j (0..10) {
        for my $k (0..10) {
            for my $l (0..10) {
                for my $m (0..10) {
                    # ... continue nesting
                }
            }
        }
    }
}

# Deeply nested subroutines
sub outer {
    my $inner = sub {
        my $inner2 = sub {
            my $inner3 = sub {
                # ... continue nesting
            };
        };
    };
}

# Deeply nested parentheses
((((((((((((((((((((($x))))))))))))))))));
```

### Current Behavior

When parsing deeply nested constructs:
- Parser may use unbounded recursion
- No explicit limit on nesting depth
- No timeout protection for pathological cases
- Stack overflow possible with extreme nesting

This results in:
- **Potential parser crashes** due to stack overflow
- **Parser hangs** on pathological input
- **No graceful degradation** for deeply nested code
- **Security vulnerability** to crafted input

### Expected Behavior

The parser should:
1. Implement explicit nesting depth limits
2. Use iterative parsing where possible instead of recursion
3. Add timeout protection for pathological cases
4. Provide graceful error messages for exceeded limits
5. Have test coverage for extreme nesting scenarios

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/parser.rs` - Add nesting depth limits and timeout protection
- `/crates/perl-parser/src/lib.rs` - Add configuration constants for limits

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/deep_nesting_risks.pl` with extreme nesting examples

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Explicit nesting depth limit (e.g., 100 levels)
   - [ ] Timeout protection for parsing operations
   - [ ] Graceful error messages for exceeded limits
   - [ ] Iterative parsing where possible

2. **Test Coverage**:
   - [ ] At least 8 test cases in corpus covering:
     - Deeply nested if/else statements
     - Deeply nested blocks
     - Deeply nested loops
     - Deeply nested subroutines
     - Deeply nested parentheses
     - Mixed deep nesting
     - Boundary testing (just under limit)
     - Error cases (exceeding limit)

3. **Security Validation**:
   - [ ] Parser handles pathological input gracefully
   - [ ] No stack overflow on extreme nesting
   - [ ] No infinite loops or hangs
   - [ ] Memory bounded for all inputs

4. **LSP Integration**:
   - [ ] Error messages are clear and actionable
   - [ ] Diagnostics show nesting depth issues
   - [ ] Hover provides helpful context

### Solution Options

#### Option 1: Comprehensive Nesting Protection (Recommended)

**Pros**:
- Complete protection against stack overflow
- Graceful degradation for pathological cases
- Clear error messages
- Configurable limits

**Cons**:
- More complex implementation
- May reject some valid but pathological code

**Implementation**:
```rust
// Add configuration constants
const MAX_NESTING_DEPTH: usize = 100;
const PARSER_TIMEOUT_MS: u64 = 5000;

// Add depth tracking to parser
struct ParserContext {
    nesting_depth: usize,
    start_time: Instant,
}

// Check depth before entering nested construct
fn enter_nested(&mut self) -> Result<(), ParseError> {
    self.context.nesting_depth += 1;
    if self.context.nesting_depth > MAX_NESTING_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth: self.context.nesting_depth,
            max_depth: MAX_NESTING_DEPTH,
        });
    }
    Ok(())
}

// Check timeout periodically
fn check_timeout(&self) -> Result<(), ParseError> {
    let elapsed = self.context.start_time.elapsed();
    if elapsed.as_millis() > PARSER_TIMEOUT_MS {
        return Err(ParseError::Timeout {
            elapsed,
            timeout: PARSER_TIMEOUT_MS,
        });
    }
    Ok(())
}
```

#### Option 2: Minimal Nesting Protection

**Pros**:
- Simpler implementation
- Faster to implement
- Reduces immediate risk

**Cons**:
- Less comprehensive protection
- May still have edge cases
- Limited configurability

**Implementation**:
```rust
// Simple depth limit
const MAX_NESTING_DEPTH: usize = 100;

// Basic depth check
fn check_depth(&self) -> Result<(), ParseError> {
    if self.nesting_depth > MAX_NESTING_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth: self.nesting_depth,
            max_depth: MAX_NESTING_DEPTH,
        });
    }
    Ok(())
}
```

#### Option 3: Iterative Parsing Only

**Pros**:
- Eliminates recursion completely
- No stack overflow risk
- Predictable memory usage

**Cons**:
- Major parser rewrite
- More complex implementation
- May be slower for normal cases

**Implementation**:
- Rewrite parser to use explicit stacks instead of recursion
- Implement all parsing constructs iteratively
- Significantly more complex

### Path Forward

**Recommended**: Option 1 (Comprehensive Nesting Protection)

**Rationale**:
1. Provides comprehensive protection against stack overflow
2. Graceful degradation for pathological cases
3. Clear error messages help developers understand issues
4. Configurable limits allow tuning for different use cases
5. Implementation complexity is manageable

**Implementation Steps**:
1. Add configuration constants for nesting depth and timeout
2. Implement depth tracking in parser context
3. Add depth checks before entering nested constructs
4. Implement periodic timeout checks
5. Add graceful error messages for exceeded limits
6. Create `test_corpus/deep_nesting_risks.pl` with extreme nesting examples
7. Validate parser handles pathological cases gracefully
8. Test with real-world Perl code (should not be affected)
9. Document limits and error messages
10. Add LSP integration tests for error diagnostics

**Timeline Estimate**: 3-5 days for implementation + 2 days for testing

### References

- **Parser Architecture**: `/crates/perl-parser/src/parser.rs` - Parser implementation
- **Error Handling**: [Error Handling Strategy Guide](docs/ERROR_HANDLING_STRATEGY.md)
- **Security Development**: [Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: None currently open
- **GA Feature Alignment**: Deep nesting is a P0 critical risk with no protection

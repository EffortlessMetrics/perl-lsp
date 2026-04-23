# Resolution Plan: P0 Deep Nesting Stack Overflow

## Issue Summary

Deep nesting constructs pose a **P0 critical risk** for parser stack overflow. When parsing deeply nested code structures (blocks, parentheses, loops, conditionals), the parser's recursive descent approach can exhaust the call stack, causing:

1. **Parser crashes**: Stack overflow terminates the parser process
2. **Denial of service**: LSP server becomes unresponsive
3. **Security vulnerability**: Malicious code can crash the language server

## Current State

### Implemented Protections

| Protection | Value | Location |
|------------|-------|----------|
| `MAX_RECURSION_DEPTH` | 128 | [`crates/perl-parser-core/src/engine/parser/mod.rs`](../../../crates/perl-parser-core/src/engine/parser/mod.rs) |
| Error type | `NestingTooDeep` | [`crates/perl-error/`](../../../crates/perl-error/) |
| Guard pattern | `with_recursion_guard()` | [`crates/perl-parser-core/src/engine/parser/helpers.rs`](../../../crates/perl-parser-core/src/engine/parser/helpers.rs) |

### Implementation Details

```rust
// From parser/mod.rs
const MAX_RECURSION_DEPTH: usize = 128;

// From parser/helpers.rs
fn check_recursion(&mut self) -> ParseResult<()> {
    self.recursion_depth += 1;
    if self.recursion_depth > MAX_RECURSION_DEPTH {
        return Err(ParseError::NestingTooDeep {
            depth: self.recursion_depth,
            max_depth: MAX_RECURSION_DEPTH,
        });
    }
    Ok(())
}

fn with_recursion_guard<T>(
    &mut self,
    f: impl FnOnce(&mut Self) -> ParseResult<T>,
) -> ParseResult<T> {
    self.check_recursion()?;
    // RAII pattern ensures exit_recursion() called on drop
}
```

### Stack Safety Calculation

```text
Typical stack frames between checks: ~20-30 (precedence parsing chain)
Maximum safe depth: 128 * 30 = ~3840 frames
OS stack limit: typically 1-8MB
Frame size: ~1-2KB
Safe frames: ~500-4000

128 was chosen to be well within safety margin.
```

## Gap Analysis

### Identified Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| Memory bounded usage | Medium | No test for memory usage at extreme depths |
| Function call nesting | Low | Some parsing paths may not increment depth counter |
| Expression nesting | Low | Complex expressions may hit limit before blocks |
| Configurable limits | Low | Users cannot adjust limits for generated code |

### Test Coverage Status

- [x] Below limit parsing succeeds
- [x] At limit parsing succeeds or fails gracefully
- [x] Above limit returns `NestingTooDeep` error
- [x] Error message includes depth information
- [x] Parser recovers after hitting limit
- [x] Performance: fails within 2 seconds
- [ ] **Memory: bounded usage at extreme depths**

## Proposed Solution

### Phase 1: Test Coverage Enhancement

**Objective**: Add missing memory bounded usage test

**Tasks**:

1. Create test for memory usage at extreme nesting depths
2. Verify parser memory stays bounded when hitting limit
3. Add assertion for maximum heap allocation

**Implementation**:

```rust
// Add to hang_risk_deep_nesting_tests.rs
#[test]
fn parser_hang_risk_memory_bounded_at_extreme_depth() {
    let depth = 10000;
    let code = generate_nested_code(depth);
    
    // Track memory before parsing
    let memory_before = get_current_memory_usage();
    
    let result = parse(&code);
    
    // Track memory after parsing
    let memory_after = get_current_memory_usage();
    
    // Memory should not grow significantly even at extreme depths
    let memory_growth = memory_after.saturating_sub(memory_before);
    assert!(memory_growth < 1024 * 1024, // <1MB growth
        "Memory should be bounded: grew by {} bytes", memory_growth);
    
    assert!(result.is_err(), "Should fail at extreme depth");
}
```

### Phase 2: Comprehensive Nesting Validation

**Objective**: Ensure all parsing paths increment depth counter

**Tasks**:

1. Audit all recursive parsing functions
2. Add `with_recursion_guard()` to any missing paths
3. Add tests for each nesting type

**Files to Audit**:

| File | Functions to Check |
|------|-------------------|
| [`expressions/hashes.rs`](../../../crates/perl-parser-core/src/engine/parser/expressions/hashes.rs) | `parse_hash_or_block_inner()` |
| [`expressions/calls.rs`](../../../crates/perl-parser-core/src/engine/parser/expressions/calls.rs) | Indirect call nesting |
| [`expressions/arrays.rs`](../../../crates/perl-parser-core/src/engine/parser/expressions/arrays.rs) | Array nesting |
| [`statements.rs`](../../../crates/perl-parser-core/src/engine/parser/statements.rs) | Statement nesting |

### Phase 3: Configurable Limits (Optional)

**Objective**: Allow users to configure `MAX_RECURSION_DEPTH`

**Tasks**:

1. Add `ParserConfig` struct with configurable limits
2. Update parser initialization to accept config
3. Add LSP configuration option for limit
4. Document configuration options

**Implementation Sketch**:

```rust
pub struct ParserConfig {
    /// Maximum recursion depth (default: 128)
    pub max_recursion_depth: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_recursion_depth: 128,
        }
    }
}

impl Parser {
    pub fn with_config(input: &str, config: ParserConfig) -> Self {
        // Use config.max_recursion_depth instead of constant
    }
}
```

## Test Plan

### Existing Tests

| Test File | Purpose |
|-----------|---------|
| [`parser_boundary_validation_tests.rs`](../../../crates/perl-parser/tests/parser_boundary_validation_tests.rs) | Tests limit at exactly 128 |
| [`parser_resource_exhaustion_tests.rs`](../../../crates/perl-parser/tests/parser_resource_exhaustion_tests.rs) | Tests behavior above limit |
| [`hang_risk_deep_nesting_tests.rs`](../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs) | Security-focused nesting tests |
| [`parser_depth_limit_test.rs`](../../../crates/perl-parser/tests/parser_depth_limit_test.rs) | Depth limit validation |
| [`parser_hardening_tests.rs`](../../../crates/perl-parser/tests/parser_hardening_tests.rs) | General hardening tests |

### New Tests Required

| Test | Purpose | Priority |
|------|---------|----------|
| `memory_bounded_at_extreme_depth` | Verify bounded memory usage | High |
| `all_nesting_types_covered` | Test all nesting patterns | Medium |
| `configurable_limits` | Test config option | Low |

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

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| `MAX_RECURSION_DEPTH` constant | ✅ Exists | Value: 128 |
| `ParseError::NestingTooDeep` | ✅ Exists | In perl-error crate |
| `with_recursion_guard()` | ✅ Exists | RAII pattern |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Stack overflow before limit | Low | Critical | Conservative limit of 128 |
| Missing recursion check | Low | High | Audit all parsing paths |
| Memory exhaustion | Low | Medium | Add memory bounded test |
| Valid code rejected | Low | Low | Limit is generous for real code |

## Action Items

### Immediate (Required)

1. [ ] Add memory bounded usage test to [`hang_risk_deep_nesting_tests.rs`](../../../crates/perl-parser/tests/hang_risk_deep_nesting_tests.rs)
2. [ ] Run full test suite: `cargo test -p perl-parser --test hang_risk_deep_nesting_tests`
3. [ ] Verify all nesting patterns are covered

### Short-term (Recommended)

1. [ ] Audit all recursive parsing functions for depth tracking
2. [ ] Add test cases for each nesting type
3. [ ] Document stack safety calculation in code comments

### Long-term (Optional)

1. [ ] Implement configurable limits via `ParserConfig`
2. [ ] Add LSP configuration option for recursion limit
3. [ ] Consider iterative parsing for extreme cases

## Conclusion

**Status: MOSTLY COMPLETE** - Core protection is implemented and tested. Missing only memory bounded usage test. The `MAX_RECURSION_DEPTH = 128` limit provides strong protection against stack overflow attacks.

## References

- [Issue Documentation](../corpus/gaps/timeout-hang-risks/deep-nesting-stack-overflow.md)
- [Parser Core Implementation](../../../crates/perl-parser-core/src/engine/parser/mod.rs)
- [Recursion Guard Helpers](../../../crates/perl-parser-core/src/engine/parser/helpers.rs)
- [CWE-674: Uncontrolled Recursion](https://cwe.mitre.org/data/definitions/674.html)

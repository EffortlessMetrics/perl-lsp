# Resolution Plan: P0 Catastrophic Regex Backtracking

## Issue Summary

Complex regex patterns pose a **P0 critical risk** for catastrophic backtracking, a class of algorithmic complexity attacks where certain regex patterns exhibit exponential time complexity on specific inputs. This can cause:

1. **Parser hangs**: Regex parsing may never complete
2. **Denial of service**: LSP server becomes unresponsive
3. **Excessive resource usage**: CPU and memory exhaustion

### Attack Classification

- **CWE-1333**: Inefficient Regular Expression Complexity
- **OWASP Category**: Denial of Service (ReDoS)

## Current State

### Implemented Protections

| Protection | Value | Location |
|------------|-------|----------|
| `MAX_REGEX_BYTES` | 64KB | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |
| `MAX_REGEX_PARSE_STEPS` | 32K | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |
| `MAX_DELIM_NEST` | 128 | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |
| `MAX_HEREDOC_DEPTH` | 100 | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |
| `MAX_HEREDOC_BYTES` | 256KB | [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) |

### Current Implementation

```rust
// From crates/perl-lexer/src/lib.rs
const MAX_REGEX_BYTES: usize = 64 * 1024;  // 64KB max for regex patterns
pub const MAX_REGEX_PARSE_STEPS: usize = 32 * 1024;
const MAX_HEREDOC_BYTES: usize = 256 * 1024; // 256KB max for heredoc bodies
const MAX_DELIM_NEST: usize = 128;         // Max nesting depth for delimiters
const MAX_HEREDOC_DEPTH: usize = 100;      // Max nesting depth for heredocs
```

### Budget Guard Behavior

- Patterns exceeding 64KB are truncated or rejected
- Delimiter nesting beyond 128 levels fails
- Emits `UnknownRest` token for graceful degradation

## Gap Analysis

### Identified Gaps

| Gap | Severity | Description |
|-----|----------|-------------|
| No pattern analysis | **High** | The lexer does not statically detect risky patterns such as nested quantifiers |
| No engine-risk diagnostics | **High** | Users do not get warnings about regex-engine catastrophic backtracking risks |
| No timeout protection | Medium | There is no separate time-based defense-in-depth budget |
| No user warnings | Medium | No LSP diagnostics surface risky regex constructs yet |

### Test Coverage Status

- [x] Byte limit enforcement (64KB)
- [x] Parse-step budget enforcement (32K)
- [x] Delimiter nesting limit (128)
- [x] Heredoc depth limit (100)
- [x] Graceful degradation to `UnknownRest`
- [ ] **Pattern analysis for nested quantifiers**
- [ ] **Risk diagnostics for pathological patterns**
- [ ] **Memory bounded during parsing**

### Pathological Patterns to Detect

| Pattern | Risk Level | Time Complexity |
|---------|------------|-----------------|
| `(a+)+` | Critical | O(2^n) |
| `(a*)*` | Critical | O(2^n) |
| `(a or aa)+` | High | O(2^n) |
| `(a?){n}` | High | O(2^n) |
| `(.*)\1` | Medium | O(n^2) |

## Proposed Solution

### Phase 1: Lexer Parse Budget Hardening (Delivered)

**Objective**: Bound regex literal scanning in the lexer before the byte budget trips

**Rationale**: The lexer now enforces a parse-step budget that is reachable before the 64KB byte ceiling, which prevents runaway literal scanning without pretending to solve regex-engine runtime backtracking.

**Tasks**:

1. Add `MAX_REGEX_PARSE_STEPS`
2. Enforce the parse-step guard in `parse_regex()`
3. Degrade to `UnknownRest` when the budget is exceeded
4. Add sub-64KB tests that prove the guard triggers before the byte budget

**Implementation**:

```rust
pub const MAX_REGEX_PARSE_STEPS: usize = 32 * 1024;

while let Some(ch) = self.current_char() {
    regex_parse_steps += 1;
    if regex_parse_steps > MAX_REGEX_PARSE_STEPS {
        self.position = self.input.len();
        return Some(Token {
            token_type: TokenType::UnknownRest,
            text: empty_arc(),
            start,
            end: self.position,
        });
    }

    if let Some(token) = self.budget_guard(start, 0) {
        return Some(token);
    }
}
```

**Files Updated in Phase 1**:

| File | Changes |
|------|---------|
| [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) | Add `MAX_REGEX_PARSE_STEPS` and enforce the parse budget |
| [`crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs`](../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs) | Add sub-64KB parse-budget tests |

### Phase 2: Pattern Analysis (Recommended)

**Objective**: Detect and warn about pathological regex patterns

**Rationale**: Static analysis can catch many dangerous patterns before parsing begins, providing early warning without runtime overhead.

**Tasks**:

1. Create regex pattern analyzer module
2. Implement nested quantifier detection
3. Implement overlapping alternative detection
4. Add LSP diagnostic emission for warnings

**Implementation**:

```rust
// New file: crates/perl-lexer/src/regex_analysis.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexRisk {
    Low,      // Safe pattern
    Medium,   // Some risk, warn user
    High,     // Dangerous, may reject
}

pub struct RegexAnalyzer;

impl RegexAnalyzer {
    /// Detect nested quantifiers like (a+)+ or (a*)*
    pub fn detect_nested_quantifiers(pattern: &str) -> bool {
        // Pattern to detect nested quantifiers
        let nested_quantifier_regex = Regex::new(r"\([^)]*[+*][^)]*\)[+*]").unwrap();
        nested_quantifier_regex.is_match(pattern)
    }
    
    /// Detect overlapping alternatives like (a|aa|aaa)+
    pub fn detect_overlapping_alternatives(pattern: &str) -> bool {
        // Check for alternatives that can match same content
        // Implementation would parse alternatives and check overlaps
        false // Placeholder
    }
    
    pub fn analyze_risk(pattern: &str) -> RegexRisk {
        if Self::detect_nested_quantifiers(pattern) {
            return RegexRisk::High;
        }
        if Self::detect_overlapping_alternatives(pattern) {
            return RegexRisk::Medium;
        }
        RegexRisk::Low
    }
}
```

**Files to Create/Modify**:

| File | Action |
|------|--------|
| [`crates/perl-lexer/src/regex_analysis.rs`](../../../crates/perl-lexer/src/regex_analysis.rs) | Create new module |
| [`crates/perl-lexer/src/lib.rs`](../../../crates/perl-lexer/src/lib.rs) | Import and use analyzer |

### Phase 3: Timeout Protection (Optional)

**Objective**: Add time-based timeout as defense-in-depth

**Rationale**: A timeout provides a safety net for any hang scenarios not caught by other protections.

**Tasks**:

1. Add `REGEX_PARSE_TIMEOUT_MS` constant
2. Track elapsed time during parsing
3. Return timeout error if exceeded
4. Add test coverage

**Implementation**:

```rust
const REGEX_PARSE_TIMEOUT_MS: u64 = 1000; // 1 second

fn parse_regex_with_timeout(&mut self) -> ParseResult<Token> {
    let start = Instant::now();
    
    // ... parsing logic with periodic checks ...
    
    if start.elapsed().as_millis() as u64 > REGEX_PARSE_TIMEOUT_MS {
        return Err(ParseError::Timeout {
            operation: "regex parsing".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        });
    }
    
    Ok(token)
}
```

### Phase 4: LSP Diagnostics (Optional)

**Objective**: Warn users about risky regex patterns via LSP

**Rationale**: Proactive warnings help users write safer code and understand potential issues.

**Tasks**:

1. Create diagnostic provider for regex patterns
2. Integrate with LSP diagnostic pipeline
3. Add configuration option to disable warnings

**Implementation**:

```rust
fn check_regex_pattern(&self, pattern: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    if RegexAnalyzer::detect_nested_quantifiers(pattern) {
        diagnostics.push(Diagnostic {
            message: "Regex pattern may cause catastrophic backtracking".to_string(),
            severity: DiagnosticSeverity::Warning,
            code: Some("nested-quantifier".to_string()),
            source: Some("perl-lsp".to_string()),
            ..Default::default()
        });
    }
    
    diagnostics
}
```

## Test Plan

### Existing Tests

| Test File | Purpose |
|-----------|---------|
| [`lexer_catastrophic_regex_test.rs`](../../../crates/perl-lexer/tests/lexer_catastrophic_regex_test.rs) | Regex parse-budget enforcement tests |
| [`hang_risk_regex_literal_tests.rs`](../../../crates/perl-lexer/tests/hang_risk_regex_literal_tests.rs) | Regex literal hang risks |

### New Tests Required

| Test | Purpose | Priority |
|------|---------|----------|
| `nested_quantifier_detected` | Verify pattern analysis | High |
| `timeout_protection_works` | Verify timeout limit | Medium |
| `risk_diagnostics_emitted` | Verify risky patterns surface diagnostics | High |

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
| Nested `/(a+)+/` | 100 chars | <100ms (limit hit) |
| Deep nesting `/(a{1}){128}/` | Any | <100ms (limit hit) |

### Validation Commands

```bash
# Run existing tests
cargo test -p perl-lexer --test lexer_catastrophic_regex_test

# Run hang risk tests
cargo test -p perl-lexer --test hang_risk_regex_literal_tests

# Run with timing assertions
cargo test -p perl-lexer -- --test-threads=1 regex
```

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| `MAX_REGEX_PARSE_STEPS` | ✅ Exists | Phase 1 lexer hardening is merged separately |
| `regex` crate | ✅ Available | For pattern analysis |
| `Instant` | ✅ Available | For timeout protection |
| LSP diagnostics pipeline | ✅ Exists | Needed to surface regex warnings |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positives in pattern analysis | Medium | Low | Make warnings configurable |
| Timeout on valid patterns | Low | Medium | Generous timeout value |
| Missing pathological patterns | Medium | High | Multiple detection methods |
| Performance overhead | Low | Low | Lazy analysis |

## Action Items

### Immediate (Required)

1. [ ] Create [`regex_analysis.rs`](../../../crates/perl-lexer/src/regex_analysis.rs) module
2. [ ] Implement nested quantifier detection
3. [ ] Add LSP diagnostics for high-risk patterns
4. [ ] Add risk-analysis regression coverage

### Short-term (Recommended)

1. [ ] Implement overlapping alternative detection
2. [ ] Evaluate timeout protection as defense-in-depth
3. [ ] Add documentation for regex best practices
4. [ ] Add telemetry for repeated parse-budget hits

### Long-term (Optional)

1. [ ] Create configuration options for diagnostics and limits
2. [ ] Consider deeper regex-engine risk modeling
3. [ ] Add user-facing suppressions for intentional high-risk patterns

## Implementation Priority

```mermaid
flowchart TD
    A[Phase 1: Backtrack Limit] --> B[Phase 2: Pattern Analysis]
    B --> C[Phase 3: Timeout Protection]
    C --> D[Phase 4: LSP Diagnostics]
    
    A -.->|Required| E[Release]
    B -.->|Recommended| E
    C -.->|Optional| F[Future Release]
    D -.->|Optional| F
```

## Conclusion

**Status: PHASE 1 COMPLETE** - Byte, nesting, and parse-step budgets are in place for lexer safety. The remaining gap is static analysis and diagnostics for regex-engine catastrophic backtracking risk.

## References

- [Issue Documentation](../corpus/gaps/timeout-hang-risks/catastrophic-regex-backtracking.md)
- [Lexer Implementation](../../../crates/perl-lexer/src/lib.rs)
- [CWE-1333: Inefficient Regular Expression Complexity](https://cwe.mitre.org/data/definitions/1333.html)
- [OWASP ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS)
- [Runaway Regular Expressions](https://www.regular-expressions.info/catastrophic.html)

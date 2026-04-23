# ADR-0029: Mutation Testing Sentinel Values

**Status**: Accepted
**Date**: 2025-02-20
**Decision Makers**: Perl LSP Architecture Team
**Related**: [CONTRIBUTING.md](../../CONTRIBUTING.md)

## Context

Mutation testing is a powerful technique for evaluating test suite quality. It introduces small changes (mutations) to the code and checks if tests detect them. However, mutation testing faces a challenge with return value mutations:

### The Return Value Problem

Consider a function that parses regex components:

```rust
fn extract_regex_parts(input: &str) -> (String, String) {
    // Complex parsing logic...
    (pattern.to_string(), modifiers.to_string())
}
```

A mutation testing tool might create these mutants:

| Mutant | Original | Mutated |
|--------|----------|---------|
| FnValue1 | `(pattern, modifiers)` | `(String::new(), String::new())` |
| FnValue2 | `(pattern, modifiers)` | `("xyzzy".into(), String::new())` |
| FnValue3 | `(pattern, modifiers)` | `("xyzzy".into(), "xyzzy".into())` |

### Detection Challenge

Tests must distinguish between:
1. **Correct behavior**: Function returns expected values
2. **Mutation**: Function returns sentinel values

Without specific assertions, mutations may survive because:
- Empty string assertions pass for `String::new()` mutants
- Generic assertions don't catch specific value mutations
- Test coverage looks good but quality is poor

## Decision

**We use specific sentinel values (`"xyzzy"`, `"sentinel"`, `"deadbeef"`) in mutation tests to detect return value mutations, ensuring these values are unlikely in real data while being memorable for developers.**

### Sentinel Value Selection

| Sentinel | Origin | Usage |
|----------|--------|-------|
| `xyzzy` | Adventure game magic word | String return values |
| `sentinel` | Generic term | Alternative string returns |
| `deadbeef` | Hex placeholder | Numeric/hex contexts |

### Why These Values?

1. **Unlikely in Real Data**:
   - `xyzzy` is a nonsense word from Colossal Cave Adventure
   - `sentinel` is a technical term rarely in user data
   - `deadbeef` is a debug pattern, not typical Perl content

2. **Memorable**:
   - Developers recognize them immediately
   - Easy to search for in code
   - Clear intent in test assertions

3. **Distinctive**:
   - Different from typical empty/string defaults
   - Stand out in test output
   - Cannot be confused with real values

### Implementation Pattern

```rust
/// Mutation hardening tests for quote parser functions
#[test]
fn test_no_sentinel_values_property() {
    // Forbidden values that indicate mutation survival
    let forbidden_values = vec!["xyzzy", "sentinel", "mutation", "deadbeef"];
    
    let test_inputs = vec![
        "", "qr", "qr/test/i", "m/pattern/", 
        "s/old/new/g", "tr/abc/xyz/"
    ];
    
    for input in test_inputs {
        // Test regex parts
        let (pattern, _, modifiers) = extract_regex_parts(input);
        assert_ne!(pattern, "xyzzy", "Pattern should not be sentinel for '{}'", input);
        assert_ne!(modifiers, "xyzzy", "Modifiers should not be sentinel for '{}'", input);
        
        // Test substitution parts
        let (pattern, replacement, modifiers) = extract_substitution_parts(input);
        assert_ne!(pattern, "xyzzy", "Sub pattern should not be sentinel for '{}'", input);
        assert_ne!(replacement, "xyzzy", "Sub replacement should not be sentinel for '{}'", input);
        assert_ne!(modifiers, "xyzzy", "Sub modifiers should not be sentinel for '{}'", input);
        
        // Test transliteration parts
        let (search, replace, modifiers) = extract_transliteration_parts(input);
        assert_ne!(search, "xyzzy", "TR search should not be sentinel for '{}'", input);
        assert_ne!(replace, "xyzzy", "TR replace should not be sentinel for '{}'", input);
        assert_ne!(modifiers, "xyzzy", "TR modifiers should not be sentinel for '{}'", input);
    }
}
```

### Mutation Killing Strategy

```rust
/// Kill mutation: extract_regex_parts -> (String::new(), "xyzzy".into())
#[test]
fn test_kill_regex_parts_string_new_xyzzy_mutation() {
    let cases = vec![
        ("qr/test/", ("/test/", "")),  // Should return pattern, not String::new()
        ("m/pattern/i", ("/pattern/", "i")), // Should return both values
    ];
    
    for (input, (expected_pattern, expected_mods)) in cases {
        let (actual_pattern, actual_mods) = extract_regex_parts(input);
        
        // Kill the mutation by checking specific values
        assert_ne!(actual_pattern, "", "Pattern should not be String::new() for '{}'", input);
        assert_ne!(actual_mods, "xyzzy", "Modifiers should not be 'xyzzy' for '{}'", input);
        
        // Also verify correct behavior
        assert_eq!(actual_pattern, expected_pattern);
        assert_eq!(actual_mods, expected_mods);
    }
}
```

### Property-Based Testing

```rust
/// Property: No function should ever return "xyzzy" in normal operation
#[test]
fn test_no_xyzzy_property() {
    let test_inputs = vec![
        "", "qr", "qr/test/i", "m/pattern/", "s/old/new/g",
        "tr/abc/xyz/", "qr{nested}i", "s(old)(new)ge",
    ];
    
    for input in test_inputs {
        let (pattern, modifiers) = extract_regex_parts(input);
        assert_ne!(pattern, "xyzzy", "Pattern should never be 'xyzzy' for '{}'", input);
        assert_ne!(modifiers, "xyzzy", "Modifiers should never be 'xyzzy' for '{}'", input);
    }
}
```

### Test File Organization

Mutation hardening tests are organized in dedicated files:

```
crates/perl-parser/tests/
├── quote_parser_mutation_hardening.rs
├── quote_parser_mutation_survivors_elimination.rs
├── quote_parser_advanced_hardening.rs
├── quote_parser_critical_mutation_elimination.rs
├── quote_parser_final_hardening.rs
├── quote_parser_realistic_hardening.rs
└── quote_parser_pr173_mutation_elimination.rs
```

## Consequences

### Positive

- **Mutation Detection**: Sentinel assertions catch return value mutations
- **Test Quality**: Forces specific value assertions, not just type checks
- **Documentation**: Sentinel values document mutation testing intent
- **Consistency**: Standardized approach across all mutation tests
- **Maintainability**: Clear pattern for adding new mutation tests

### Negative

- **Test Verbosity**: Additional assertions increase test code size
- **False Confidence**: Only catches known mutation patterns
- **Maintenance**: Must update tests when function signatures change
- **Learning Curve**: New contributors must understand the pattern

### Mitigations

- Helper macros to reduce boilerplate
- Clear documentation in CONTRIBUTING.md
- Code review checklist for mutation test coverage
- Automated mutation testing in CI

## References

- [crates/perl-parser/tests/quote_parser_mutation_hardening.rs](../../crates/perl-parser/tests/quote_parser_mutation_hardening.rs) - Mutation tests
- [crates/perl-lsp-rs/tests/mutation_survivors_elimination.rs](../../crates/perl-lsp-rs/tests/mutation_survivors_elimination.rs) - LSP mutation tests
- [crates/perl-lsp-rs/tests/critical_mutation_hardening.rs](../../crates/perl-lsp-rs/tests/critical_mutation_hardening.rs) - Critical mutation tests
- [CONTRIBUTING.md](../../CONTRIBUTING.md) - Contribution guidelines
- [cargo-mutants](https://github.com/sourcegraph/cargo-mutants) - Mutation testing tool

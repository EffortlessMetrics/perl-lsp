# Perl LSP Test Coverage Plan

**Date:** 2026-04-09  
**Context:** 113+ issues filed, architectures designed. Need comprehensive test coverage for fixes.  
**Author:** Hearth (subagent research)

---

## Executive Summary

The perl-lsp codebase has **979+ test files** across a **137-crate workspace**. While the project has extensive happy-path and unhappy-path coverage (243+ tests in parser alone per `FINAL_TEST_COVERAGE_REPORT.md`), there are specific architectural gaps that need addressing to make the 113+ filed issues regression-safe.

This plan identifies coverage gaps and provides test strategies for:
- **BUILTIN_INITIALIZERS registry** — tracking which built-ins initialize variables
- **EffectiveSemantics layer** — the scope analyzer's pragma-aware semantic analysis
- **Parser deref_base annotation** — dereference sigil-to-variable bridging
- **Regression tests for P0 issues** — #3336, #3337, #3338, #3339

---

## 1. Current Test Coverage Assessment

### 1.1 What Exists (Strong Foundation)

| Area | Coverage | Test Pattern |
|------|----------|--------------|
| `perl-lsp-diagnostics` | 13 test files | Helper-based: `diagnostics_for(source)` → filter by code |
| `perl-semantic-analyzer` | 11 test files | Unit + integration with mock ASTs |
| `perl-parser` | 979+ test files | Comprehensive e2e, happy/unhappy path |
| Dead code detection | ✅ Covered | `detect_dead_code` integration tests |
| Bareword diagnostics | ✅ Covered | `bareword_diagnostics_tests.rs` — pattern to follow |
| Version compatibility | Partial | `version_compat_tests.rs` exists but needs expansion |

### 1.2 Existing Test Pattern (From `bareword_diagnostics_tests.rs`)

```rust
// Standard helper pattern used across diagnostic tests
fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn filter_by_code(diags: Vec<Diagnostic>, code: &str) -> Vec<Diagnostic> {
    diags.into_iter().filter(|d| d.code.as_deref() == Some(code)).collect()
}

// Test structure: Arrange → Assert on specific diagnostic codes
#[test]
fn bareword_under_strict_is_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = FOO;\n";
    let diags = bareword_diags(source);
    assert!(!diags.is_empty(), "expected at least one unquoted-bareword diagnostic");
    Ok(())
}
```

### 1.3 Coverage Gaps Identified

| Gap | Location | Impact |
|-----|----------|--------|
| **BUILTIN_INITIALIZERS** | `scope_analyzer.rs` lacks explicit registry | `open my $fh, ...` flagged as undefined |
| **EffectiveSemantics** | No isolated tests for pragma→semantic mapping | `use v5.40` doesn't suppress strict |
| **deref_base annotation** | Parser marks derefs but tests don't verify | `@$arrayref` doesn't mark `$arrayref` used |
| **Version→Pragma mapping** | Incomplete in `pragma_tracker.rs` | False positives on modern Perl |

---

## 2. Test Architecture Gaps

### 2.1 Missing: BUILTIN_INITIALIZERS Registry

**Current State:** `scope_analyzer.rs` has ad-hoc handling for built-in initializers scattered in the analysis logic.

**Needed:** A centralized, testable registry that declares which built-ins initialize their arguments:

```rust
// Proposed: crates/perl-semantic-analyzer/src/analysis/builtin_initializers.rs
pub const BUILTIN_INITIALIZERS: &[(&str, InitPattern)] = &[
    ("open", InitPattern::FirstArg),
    ("sysopen", InitPattern::FirstArg),
    ("pipe", InitPattern::FirstTwoArgs),
    ("socket", InitPattern::FirstArg),
    ("socketpair", InitPattern::FirstTwoArgs),
    ("accept", InitPattern::FirstArg),
];
```

**Test Coverage Needed:**
| Built-in | Pattern | Test Count |
|----------|---------|------------|
| `open` | `open my $fh, ...` | 4 (bare, paren, or-die, or-return) |
| `sysopen` | `sysopen my $fh, ...` | 2 |
| `pipe` | `pipe my $r, my $w` | 3 (both, first only, second only) |
| `socket` | `socket my $sock, ...` | 2 |
| `socketpair` | `socketpair my $s1, my $s2, ...` | 3 |
| `accept` | `accept my $client, $server` | 2 |
| **Total** | | **~16 tests** |

### 2.2 Missing: EffectiveSemantics Integration Tests

**Current State:** `pragma_tracker.rs` tracks pragma state; `scope_analyzer.rs` uses it for strict mode checks. No isolated tests exist for the mapping between version declarations and effective semantics.

**Needed:** Tests that verify `use v5.XX` produces correct `PragmaState`:

```rust
// Test matrix for version→pragma mapping
| Version | strict | warnings | Test |
|---------|--------|----------|------|
| v5.10 | ❌ | ❌ | `test_v5_10_no_strict` |
| v5.11 | ✅ | ❌ | `test_v5_11_strict` |
| v5.12-v5.34 | ✅ | ❌ | `test_v5_12_to_34_strict` |
| v5.35+ | ✅ | ✅ | `test_v5_35_strict_warnings` |
| v5.40 | ✅ | ✅ | `test_v5_40_complete` |
```

### 2.3 Missing: Dereference Sigil→Variable Bridging

**Current State:** `scope_analyzer.rs` has logic (around line ~340) that attempts to bridge dereference patterns to mark base variables as used, but lacks comprehensive tests.

**Needed:** Tests for each sigil combination:

| Operation | Declaration | Usage Pattern |
|-----------|-------------|---------------|
| Array deref | `my $ref` | `@$ref` |
| Hash deref | `my $ref` | `%$ref` |
| Scalar deref | `my $ref` | `$$ref` |
| Code deref | `my $ref` | `&$ref` |
| Glob deref | `my $ref` | `*$ref` |
| Ref create | `my $ref` | `\$ref` |

Plus braced variants: `@{$ref}`, `@{ $ref }`, nested: `@{ $ref->[0] }`.

---

## 3. Recommended Test File Structure

```
crates/perl-lsp-diagnostics/tests/
├── unit_tests.rs                    # ✅ Existing — core diagnostic types
├── bareword_diagnostics_tests.rs    # ✅ Existing — pattern to follow
├── dead_code_diagnostics_test.rs   # ✅ Existing
├── version_compat_tests.rs          # ⚠️ Partial — needs expansion
├── builtin_initializer_tests.rs     # 🆕 NEW — ~16 tests for #3336
├── sigil_dereference_tests.rs       # 🆕 NEW — ~25 tests for #3338
└── version_pragma_tests.rs          # 🆕 NEW — ~20 tests for #3339

crates/perl-semantic-analyzer/tests/
├── builtin_context_docs_tests.rs    # ✅ Existing
├── scope_and_symbol_tests.rs        # ⚠️ Partial — needs initializer tests
└── effective_semantics_tests.rs     # 🆕 NEW — pragma→semantic mapping

crates/perl-module-resolution-path/tests/
└── workspace_relative_tests.rs      # 🆕 NEW — config path resolution for #3337
```

---

## 4. Test Plan for Top 5 P0 Issues

### Issue #3336: Builtin Initializers (`open my $fh, ...`)

**Problem:** Variables initialized by built-ins like `open` are flagged as undefined.

**Test File:** `crates/perl-lsp-diagnostics/tests/builtin_initializer_tests.rs`

**Test Cases:**

```rust
use std::sync::Arc;
use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn var_issues_for(diags: &[Diagnostic], var_name: &str) -> Vec<&Diagnostic> {
    diags.iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL103") | Some("PL110")))
        .filter(|d| d.message.contains(var_name))
        .collect()
}

// === Core Tests ===

#[test]
fn test_open_initializes_fh() {
    let source = "open my $fh, '<', 'file.txt' or die;\nprint <$fh>;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$fh").is_empty(), 
        "open my \$fh should initialize \$fh");
}

#[test]
fn test_open_parenthesized_initializes() {
    let source = "open(my $fh, '<', 'file.txt') or die;\nprint <$fh>;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$fh").is_empty());
}

#[test]
fn test_open_or_die_still_initializes() {
    // Even with 'or die', $fh is initialized before die could trigger
    let source = "open my $fh, '<', 'file.txt' or die;\nprint <$fh>;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$fh").is_empty());
}

#[test]
fn test_pipe_initializes_both_ends() {
    let source = "pipe my $r, my $w;\nprint $r;\nprint $w;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$r").is_empty());
    assert!(var_issues_for(&diags, "$w").is_empty());
}

#[test]
fn test_sysopen_initializes_fh() {
    let source = "use Fcntl;\nsysopen my $fh, 'file.txt', O_RDONLY or die;\nprint <$fh>;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$fh").is_empty());
}

#[test]
fn test_socket_initializes_sock() {
    let source = "use Socket;\nsocket my $sock, PF_INET, SOCK_STREAM, 0 or die;\nprint $sock;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$sock").is_empty());
}

#[test]
fn test_socketpair_initializes_both() {
    let source = "use Socket;\nsocketpair my $s1, my $s2, AF_UNIX, SOCK_STREAM, 0;\nprint $s1;\nprint $s2;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$s1").is_empty());
    assert!(var_issues_for(&diags, "$s2").is_empty());
}

#[test]
fn test_accept_initializes_client() {
    let source = "accept my $client, $server;\nprint $client;\n";
    let diags = diagnostics_for(source);
    assert!(var_issues_for(&diags, "$client").is_empty());
}

// === Negative Cases ===

#[test]
fn test_open_bareword_no_variable_init() {
    // open FH, ... uses bareword filehandle — no lexical variable
    let source = "open FH, '<', 'file.txt';\nprint <FH>;\n";
    let diags = diagnostics_for(source);
    // Should not trigger PL103 for FH (bareword, not a var)
    let undef: Vec<_> = diags.iter()
        .filter(|d| d.code.as_deref() == Some("PL103"))
        .filter(|d| d.message.contains("FH"))
        .collect();
    assert!(undef.is_empty(), "Bareword filehandle should not trigger undefined var");
}

#[test]
fn test_unrelated_function_no_init() {
    // close() should NOT mark its arg as initialized
    let source = "close my $fh;\nprint $fh;\n";
    let diags = diagnostics_for(source);
    // This is tricky: $fh used after close is actually a usage,
    // but close doesn't initialize. If $fh was never declared,
    // it should be flagged. If declared, just marked used.
    // This test verifies close() doesn't have initializer semantics.
}
```

---

### Issue #3338: Dereference Sigil Bridging (`@$arrayref`)

**Problem:** Dereferencing a variable doesn't mark it as used (triggers false "unused variable" diagnostic).

**Test File:** `crates/perl-lsp-diagnostics/tests/sigil_dereference_tests.rs`

**Test Cases:**

```rust
fn unused_var_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter()
        .filter(|d| d.code.as_deref() == Some("PL102"))
        .collect()
}

// === Core Sigil Tests ===

#[test]
fn test_array_deref_marks_variable_used() {
    let source = "my $arrayref;\npush @$arrayref, 'item';\n";
    let diags = unused_var_diags(&source);
    let unused: Vec<_> = diags.iter()
        .filter(|d| d.message.contains("$arrayref"))
        .collect();
    assert!(unused.is_empty(), "@\$arrayref should mark \$arrayref as used");
}

#[test]
fn test_hash_deref_marks_variable_used() {
    let source = "my $hashref;\nkeys %$hashref;\n";
    let diags = unused_var_diags(&source);
    let unused: Vec<_> = diags.iter()
        .filter(|d| d.message.contains("$hashref"))
        .collect();
    assert!(unused.is_empty(), "%\$hashref should mark \$hashref as used");
}

#[test]
fn test_scalar_deref_marks_variable_used() {
    let source = "my $ref;\nmy $val = $$ref;\n";
    let diags = unused_var_diags(&source);
    let unused: Vec<_> = diags.iter()
        .filter(|d| d.message.contains("$ref"))
        .collect();
    assert!(unused.is_empty(), "\$\$ref should mark \$ref as used");
}

#[test]
fn test_code_deref_marks_variable_used() {
    let source = "my $coderef;\n&$coderef();\n";
    let diags = unused_var_diags(&source);
    let unused: Vec<_> = diags.iter()
        .filter(|d| d.message.contains("$coderef"))
        .collect();
    assert!(unused.is_empty(), "&\$coderef should mark \$coderef as used");
}

// === Braced Variants ===

#[test]
fn test_braced_array_deref() {
    let source = "my $ref;\npush @{$ref}, 'item';\n";
    let diags = unused_var_diags(&source);
    assert!(!diags.iter().any(|d| d.message.contains("$ref")));
}

#[test]
fn test_braced_array_deref_with_spaces() {
    let source = "my $ref;\npush @{ $ref }, 'item';\n";
    let diags = unused_var_diags(&source);
    assert!(!diags.iter().any(|d| d.message.contains("$ref")));
}

// === Nested Dereferences ===

#[test]
fn test_nested_array_deref_from_arrayref() {
    let source = "my $ref;\nfor (@{ $ref->[0] }) { print; }\n";
    let diags = unused_var_diags(&source);
    assert!(!diags.iter().any(|d| d.message.contains("$ref")));
}

#[test]
fn test_nested_hash_deref_from_hashref() {
    let source = "my $ref;\nwhile (my ($k, $v) = each %{ $ref->{items} }) { }\n";
    let diags = unused_var_diags(&source);
    assert!(!diags.iter().any(|d| d.message.contains("$ref")));
}

// === Negative Cases (Element Access ≠ Dereference) ===

#[test]
fn test_element_access_is_not_deref() {
    // $array[0] should NOT mark @array as used — different variable family
    // This test ensures we don't conflate element access with deref
    let source = "my @array;\nprint $array[0];\n";
    let diags = diagnostics_for(source);
    // @array IS used (via element access), so no PL102 expected
    assert!(!diags.iter().any(|d| {
        d.code.as_deref() == Some("PL102") && d.message.contains("@array")
    }));
}
```

---

### Issue #3339: Version→Pragma Mapping (`use v5.40`)

**Problem:** `use v5.40` should implicitly enable strict and warnings, but currently doesn't suppress related diagnostics.

**Test File:** `crates/perl-lsp-diagnostics/tests/version_pragma_tests.rs`

**Test Cases:**

```rust
fn strict_warnings_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL100") | Some("PL101")))
        .collect()
}

// === Version Threshold Tests ===

#[test]
fn test_v5_10_no_implicit_strict() {
    let source = "use v5.10;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
        "v5.10 should warn about missing strict");
}

#[test]
fn test_v5_11_implicit_strict() {
    let source = "use v5.11;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(!diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
        "v5.11 should NOT warn about missing strict");
    assert!(diags.iter().any(|d| d.code.as_deref() == Some("PL101")),
        "v5.11 should still warn about missing warnings");
}

#[test]
fn test_v5_12_implicit_strict() {
    let source = "use v5.12;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(!diags.iter().any(|d| d.code.as_deref() == Some("PL100")));
}

#[test]
fn test_v5_35_implicit_strict_and_warnings() {
    let source = "use v5.35;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.is_empty(), "v5.35 should enable both strict and warnings");
}

#[test]
fn test_v5_40_no_strict_warning() {
    let source = "use v5.40;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.is_empty(), "v5.40 should enable both strict and warnings");
}

// === Dev Version Edge Cases ===

#[test]
fn test_dev_version_v5_012_001() {
    let source = "use v5.012_001;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(!diags.iter().any(|d| d.code.as_deref() == Some("PL100")),
        "Dev version v5.012_001 should enable strict");
}

#[test]
fn test_dev_version_v5_035_001() {
    let source = "use v5.035_001;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.is_empty(), "Dev version v5.035_001 should enable strict+warnings");
}

// === Format Variants ===

#[test]
fn test_decimal_three_digit_format() {
    let source = "use 5.040;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.is_empty(), "Decimal format 5.040 should work");
}

#[test]
fn test_decimal_two_digit_format() {
    let source = "use 5.40;\nmy $x = 1;\n";
    let diags = strict_warnings_diags(source);
    assert!(diags.is_empty(), "Decimal format 5.40 should work");
}

// === Explicit Override Tests ===

#[test]
fn test_no_strict_overrides_version() {
    let source = "use v5.40;\nno strict;\n$undeclared = 1;\n";
    let diags = diagnostics_for(source);
    // Should NOT report PL103 (undeclared) since strict is disabled
    assert!(!diags.iter().any(|d| d.code.as_deref() == Some("PL103")));
}

#[test]
fn test_explicit_strict_with_version() {
    let source = "use v5.40;\nuse strict;\n";
    let diags = strict_warnings_diags(source);
    // Redundant but valid — should not produce duplicate diagnostics
    assert!(diags.iter().filter(|d| d.code.as_deref() == Some("PL100")).count() <= 1);
}
```

---

### Issue #3337: includePaths Semantics

**Problem:** Module resolution configuration semantics need verification.

**Test File:** `crates/perl-module-resolution-path/tests/workspace_relative_tests.rs`

**Test Cases:**

```rust
// Unit tests for path resolution logic
#[test]
fn test_workspace_relative_resolution() {
    // Given workspace at /project with lib/MyMod.pm
    // When resolving MyMod with includePaths = ["lib"]
    // Then should resolve to /project/lib/MyMod.pm
}

#[test]
fn test_external_include_paths() {
    // Given external path /external/lib with External/Mod.pm
    // When externalIncludePaths = ["/external/lib"]
    // Then External::Mod should resolve
}

#[test]
fn test_system_inc_flag() {
    // Given useSystemInc = true
    // When resolving a core module like List::Util
    // Then should check system @INC
}
```

**Integration Tests:** `crates/perl-lsp-diagnostics/tests/module_resolution_diagnostic_tests.rs`

```rust
#[test]
fn test_core_module_not_flagged() {
    let source = "use strict;\nuse List::Util qw(sum);\n";
    let diags = diagnostics_for(source);
    assert!(!diags.iter().any(|d| d.code.as_deref() == Some("PL701")));
}

#[test]
fn test_missing_module_flagged() {
    let source = "use strict;\nuse NonExistent::Module;\n";
    let diags = diagnostics_for(source);
    assert!(diags.iter().any(|d| d.code.as_deref() == Some("PL701")));
}
```

---

## 5. Summary: Test Count by Issue

| Issue | Area | New Tests | Priority |
|-------|------|-----------|----------|
| #3336 | Builtin Initializers | ~21 | P0 |
| #3338 | Dereference Bridging | ~25 | P0 |
| #3339 | Version→Pragma | ~20 | P0 |
| #3337 | Config/Resolution | ~10 | P1 |
| **Total** | | **~76** | |

---

## 6. Implementation Checklist

### Phase 1: Core P0 Tests (Immediate)
- [ ] Create `builtin_initializer_tests.rs` with 21 tests
- [ ] Create `sigil_dereference_tests.rs` with 25 tests
- [ ] Create `version_pragma_tests.rs` with 20 tests

### Phase 2: Integration Tests
- [ ] Create `module_resolution_diagnostic_tests.rs` for #3337
- [ ] Add `effective_semantics_tests.rs` to semantic-analyzer

### Phase 3: Infrastructure
- [ ] Add `BUILTIN_INITIALIZERS` registry to semantic-analyzer
- [ ] Ensure all failing tests are marked `#[ignore = "waiting for fix #XXXX"]`
- [ ] As fixes land, remove ignore attributes

### Phase 4: Coverage Verification
- [ ] Run `cargo test --workspace` and verify all new tests pass
- [ ] Add CI check: `cargo test --workspace -- --ignored` to catch ignored tests

---

## 7. Pattern to Follow

All new tests should follow this established pattern from `bareword_diagnostics_tests.rs`:

1. **Use `diagnostics_for(source)` helper** — Parses and runs full diagnostic pipeline
2. **Filter by diagnostic code** — Not message (codes are stable)
3. **Assert on specific outcomes** — Empty, contains, count, etc.
4. **Test both positive and negative cases** — What should and shouldn't trigger
5. **Return `Result<(), Box<dyn std::error::Error>>`** — For ergonomic error handling

---

*End of Test Plan*

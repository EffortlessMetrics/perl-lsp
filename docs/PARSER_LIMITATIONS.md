# Parser Limitations

This document details intentional parser boundaries and historically resolved issues in the perl-parser.

## Overview

The perl-parser (v3 Native) has **~100% Perl syntax coverage** for static Perl code. This document distinguishes between:

1. **Intentional Boundaries**: Features that require runtime evaluation or are explicitly out of scope
2. **Resolved Issues**: Previously tracked parser limitations that have been fixed

---

## 1. Intentional Boundaries (Non-Goals)

These are not bugs—they represent fundamental limits of static analysis for a dynamic language.

### Source Filters

**Scope**: Out of scope for static parsing

**Description**: Source filters (`Filter::Simple`, `Filter::Util::Call`, etc.) transform Perl source code at compile time before parsing. This requires actually executing Perl code.

**Examples**:
```perl
use Switch;      # Modifies source before parsing
use Perl6::Say;  # Adds 'say' keyword via source filter
```

**Workaround**: Users should review files using source filters manually or with Perl's own tools.

### eval STRING

**Scope**: Cannot analyze dynamically-constructed code

**Description**: `eval STRING` compiles and executes Perl code at runtime. The parser cannot know what code will be generated.

**Examples**:
```perl
my $code = build_code();  # Dynamic code construction
eval $code;                # Cannot be statically analyzed
```

**What We Do**: We parse the `eval` statement itself correctly, but cannot analyze the evaluated code.

### Dynamic Symbol Table Manipulation

**Scope**: Runtime behavior cannot be predicted

**Description**: Perl allows dynamic manipulation of symbol tables, stash entries, and glob assignments. These change program behavior at runtime.

**Examples**:
```perl
*foo = sub { ... };              # Dynamic sub definition
no strict 'refs'; *{$name} = 1;  # Dynamic variable creation
```

**What We Do**: We parse these constructs syntactically but cannot determine their runtime effects.

### BEGIN Block Side Effects

**Scope**: Compile-time effects require execution

**Description**: `BEGIN` blocks execute during compilation and can modify the compilation environment in arbitrary ways.

**Examples**:
```perl
BEGIN {
    require Some::Module;    # Loads module at compile time
    *func = \&Some::func;    # Modifies symbol table
}
```

**What We Do**: We parse `BEGIN` blocks but don't execute them.

---

## 2. Resolved Issues (Historical)

These were previously tracked as parser limitations and have been fixed.

### Return Statement After Word Operators - RESOLVED ✅

**PR**: #261
**Previous Issue**: `$a = 1 or return` didn't parse correctly because `return` was treated as a statement rather than expression.

**Resolution**: Improved operator precedence handling to recognize `return` as valid expression in low-precedence operator contexts.

**Validation**:
```bash
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations
```

---

### Indirect Object Syntax Detection - RESOLVED ✅

**PR**: #261
**Previous Issue**: `print $fh $x;` was not recognized as indirect object form.

**Resolution**: Improved heuristics for detecting indirect object syntax in common patterns (`print`, `say`, `new`, `open`).

**Validation**:
```bash
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect
```

---

### Whitespace Insertion Algorithm - RESOLVED ✅

**PR**: #261
**Previous Issue**: The `insertion_safe` algorithm had non-deterministic edge cases discovered through property-based testing.

**Resolution**: Made the algorithm deterministic by defining stable ordering for iteration.

**Validation**:
```bash
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent
```

---

### Malformed Substitution Strictness - RESOLVED ✅

**PR**: #261
**Previous Issue**: Malformed substitution operators (`s/pattern/`) didn't consistently produce errors.

**Resolution**: Improved substitution operator validation to properly reject malformed patterns.

**Validation**:
```bash
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed
```

---

## Summary

| Category | Count | Notes |
|----------|-------|-------|
| **Intentional Boundaries** | 4 | Source filters, eval STRING, dynamic symbols, BEGIN effects |
| **Resolved (PR #261)** | 4 | Return precedence, indirect objects, whitespace, substitution |

## Related Documentation

- **[IGNORED_TESTS_ROADMAP.md](IGNORED_TESTS_ROADMAP.md)**: Test tracking and resolution status
- **[Issue #188](https://github.com/EffortlessMetrics/perl-lsp/issues/188)**: Semantic Analyzer (for deeper analysis needs)
- **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)**: LSP feature coverage

## Version History

- **v0.8.8** (2025-01-31): Initial documentation
- **v0.8.9** (2025-12-31): Refactored to show resolved issues (PR #261)

# Parser Limitations

This document details intentional parser boundaries and historically resolved issues in the perl-parser.

## Overview

The perl-parser (v3 Native) has **~100% Perl syntax coverage** for static Perl code. This document distinguishes between:

1. **Intentional Boundaries**: Features that require runtime evaluation or are explicitly out of scope
2. **Resolved Issues**: Previously tracked parser limitations that have been fixed

**For comprehensive feature comparison and version-specific limitations, see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md).**

---

## 1. Intentional Boundaries (Non-Goals)

These are not bugs—they represent fundamental limits of static analysis for a dynamic language.

### 1.1 Source Filters

**Scope**: Out of scope for static parsing

**Description**: Source filters (`Filter::Simple`, `Filter::Util::Call`, etc.) transform Perl source code at compile time before parsing. This requires actually executing Perl code.

**Examples**:
```perl
use Switch;      # Modifies source before parsing
use Perl6::Say;  # Adds 'say' keyword via source filter
```

**Technical Details**:
```perl
# Filter::Simple example - transforms source before parsing
use Filter::Simple;
FILTER {
    s/Hello/Goodbye/g;
    s/good/bad/g;
}

# After filter runs, this code:
print "Hello, good morning!";

# Is transformed to:
print "Goodbye, bad morning!";
```

**Common Source Filter Modules**:
| Module | Purpose | Impact on Static Analysis |
|--------|---------|---------------------------|
| `Switch` | Adds switch/case syntax | Changes control flow syntax |
| `Perl6::Say` | Adds 'say' keyword | Adds new keyword |
| `Filter::Simple` | Custom transformations | Arbitrary code changes |
| `Spiffy` | Module framework | Changes OO syntax |
| `B::Hooks::OP::Check` | OP manipulation | Runtime behavior changes |

**Workaround**: Users should review files using source filters manually or with Perl's own tools:
```bash
# Check if a file uses source filters
perl -MO=Deparse file.pl

# Or check compilation
perl -c file.pl
```

**Related**: See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md#1-source-filters) for workarounds.

---

### 1.2 eval STRING

**Scope**: Cannot analyze dynamically-constructed code

**Description**: `eval STRING` compiles and executes Perl code at runtime. The parser cannot know what code will be generated.

**Examples**:
```perl
# Dynamic code from variable
my $code = build_code();  # Dynamic code construction
eval $code;                # Cannot be statically analyzed

# String interpolation in eval
my $sub_name = 'dynamic_' . $type;
eval "sub $sub_name { return 42; }";

# HERE-doc in eval (complex)
eval "print <<EOF;\n" . $content . "\nEOF";
```

**What We Parse vs What We Cannot**:
```perl
# This statement IS parsed:
eval $code;

# The AST correctly shows:
# (eval_expression (variable) ...)

# But the CONTENT of $code is unknown at parse time
```

**Common Patterns That Cannot Be Analyzed**:
```perl
# Dynamic subroutine creation
eval "sub $name { $body }";

# Conditional code loading
eval "require $module";

# Configuration-driven code
my $config = read_config();
eval $config->{code};

# Metaprogramming
for my $method (qw(get set delete)) {
    eval "sub $method { ... }";
}
```

**Workaround**: Use subroutine references or dispatch tables instead:
```perl
# Instead of eval STRING:
my $handler = $handlers{$action};  # Hash of code refs
$handler->(@args);

# Or use can() for method lookup:
if (my $method = $object->can($action)) {
    $object->$method(@args);
}
```

**Related**: See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md#2-runtime-code-generation) for more workarounds.

---

### 1.3 Dynamic Symbol Table Manipulation

**Scope**: Runtime behavior cannot be predicted

**Description**: Perl allows dynamic manipulation of symbol tables, stash entries, and glob assignments. These change program behavior at runtime.

**Examples**:
```perl
# Dynamic sub definition via typeglob
*foo = sub { return "Dynamic foo" };

# Dynamic variable creation
no strict 'refs';
*{$name} = 1;

# Symbol aliasing
*alias = *original;

# Package manipulation via stash
$main::{$subname} = sub { ... };

# Runtime method injection
*Class::method = sub { ... };
```

**Technical Details**:
```perl
# Symbol table structure
# %Package:: is the symbol table (stash)
# Each entry is a typeglob (*symbol)
# Typeglobs can contain: SCALAR, ARRAY, HASH, CODE, IO, FORMAT

# This means at runtime:
*foo = *bar;      # foo is now an alias for bar
*foo = \$scalar;  # foo is now a scalar alias
*foo = \&func;    # foo is now a subroutine alias

# Static analysis cannot predict these effects
```

**What We Do**: Parse these constructs syntactically but cannot determine their runtime effects.

**Impact on LSP Features**:
| Feature | Impact |
|---------|--------|
| Go to Definition | May not find dynamically created symbols |
| Find References | May miss dynamic references |
| Completion | May not suggest dynamic symbols |
| Rename | May not update dynamic references |

**Related**: See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md#3-dynamic-symbol-table-manipulation) for more details.

---

### 1.4 BEGIN Block Side Effects

**Scope**: Compile-time effects require execution

**Description**: `BEGIN` blocks execute during compilation and can modify the compilation environment in arbitrary ways.

**Examples**:
```perl
# Module loading at compile time
BEGIN {
    require Some::Module;    # Loads module at compile time
    *func = \&Some::func;    # Modifies symbol table
}

# Conditional compilation
BEGIN {
    if ($ENV{DEBUG}) {
        *log = sub { print @_ };
    } else {
        *log = sub { };
    }
}

# Version checking at compile time
BEGIN {
    die "Perl 5.38 required" if $] < 5.038;
}

# Compile-time code generation
BEGIN {
    my $code = generate_accessors();
    eval $code;
}
```

**What BEGIN Blocks Can Affect**:
```perl
# Symbol table modifications
BEGIN { *prefix = sub { "prefix_" . shift } }

# Pragma effects
BEGIN { use strict; use warnings; }

# Constant definitions
BEGIN { use constant DEBUG => 1; }

# Conditional exports
BEGIN {
    push @EXPORT, 'debug_func' if DEBUG;
}
```

**What We Do**: Parse `BEGIN` blocks but don't execute them. Symbol table modifications within BEGIN blocks won't be reflected in static analysis.

**Related**: See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md#4-begin-block-side-effects) for more details.

---

### 1.5 Tied Variables and Filehandles

**Scope**: Custom behavior requires runtime

**Description**: Tied variables and filehandles have custom behavior defined at runtime.

**Examples**:
```perl
# Tied hash
tie %cache, 'Tie::StdHash';

# Tied filehandle
tie *FH, 'Package';

# Custom tie class
tie my $counter, 'Counter', start => 0;
```

**Impact**: Static analysis cannot determine the actual behavior of tied variables.

---

### 1.6 Operator Overloading

**Scope**: Custom operator behavior requires runtime type information

**Examples**:
```perl
package MyNumber;
use overload '+' => \&add;

sub add {
    my ($self, $other) = @_;
    # Custom addition logic
}
```

**Impact**: Static analysis cannot predict how overloaded operators will behave.

---

## 2. Resolved Issues (Historical)

These were previously tracked as parser limitations and have been fixed.

### 2.1 Return Statement After Word Operators - RESOLVED ✅

**PR**: #261

**Previous Issue**: `$a = 1 or return` didn't parse correctly because `return` was treated as a statement rather than expression.

**Example of Previous Failure**:
```perl
# Previously failed to parse correctly:
$a = 1 or return;
$b = 2 and return $b;
```

**Resolution**: Improved operator precedence handling to recognize `return` as valid expression in low-precedence operator contexts.

**Validation**:
```bash
cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations
```

---

### 2.2 Indirect Object Syntax Detection - RESOLVED ✅

**PR**: #261

**Previous Issue**: `print $fh $x;` was not recognized as indirect object form.

**Example of Previous Failure**:
```perl
# Previously not recognized:
print $fh "Hello";
say STDOUT "message";
new Class::Name;
```

**Resolution**: Improved heuristics for detecting indirect object syntax in common patterns (`print`, `say`, `new`, `open`).

**Note**: This is only resolved in v3 Native parser. v1 and v2 still have limitations.

**Validation**:
```bash
cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect
```

---

### 2.3 Whitespace Insertion Algorithm - RESOLVED ✅

**PR**: #261

**Previous Issue**: The `insertion_safe` algorithm had non-deterministic edge cases discovered through property-based testing.

**Resolution**: Made the algorithm deterministic by defining stable ordering for iteration.

**Validation**:
```bash
cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent
```

---

### 2.4 Malformed Substitution Strictness - RESOLVED ✅

**PR**: #261

**Previous Issue**: Malformed substitution operators (`s/pattern/`) didn't consistently produce errors.

**Example of Previous Behavior**:
```perl
# Previously might not error:
s/pattern/;      # Missing closing delimiter
s{unclosed;      # Missing closing bracket
```

**Resolution**: Improved substitution operator validation to properly reject malformed patterns.

**Validation**:
```bash
cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed
```

---

### 2.5 Empty Block Parsing - RESOLVED ✅

**PR**: #261 (v0.7.1)

**Previous Issue**: Empty blocks in `bless {}`, `sort {}`, `map {}`, `grep {}` were not parsed correctly.

**Example of Previous Failure**:
```perl
# Previously failed:
my $obj = bless {};           # Empty hashref for blessing
@sorted = sort {} @list;      # Empty sort block
@mapped = map {} @list;       # Empty map block
@grepped = grep {} @list;     # Empty grep block
```

**Resolution**: Enhanced builtin function argument handling to correctly parse empty blocks.

**Validation**:
```bash
cargo test -p perl-parser --test builtin_function_tests
```

---

## Summary

| Category | Count | Notes |
|----------|:-----:|-------|
| **Intentional Boundaries** | 6 | Source filters, eval STRING, dynamic symbols, BEGIN effects, tied variables, operator overloading |
| **Resolved Issues** | 5 | Return precedence, indirect objects, whitespace, substitution, empty blocks |

## Decision Matrix

When encountering a parsing limitation, use this decision tree:

```
Is the limitation documented here?
│
├─ Yes → Is it an Intentional Boundary?
│         │
│         ├─ Yes → This is a fundamental limit of static analysis
│         │         Consider: runtime tools, code refactoring
│         │
│         └─ No → Is it in Resolved Issues?
│                   │
│                   ├─ Yes → Update to latest parser version
│                   │
│                   └─ No → File a bug report
│
└─ No → File a bug report with reproduction case
```

## Related Documentation

- **[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)**: Comprehensive feature comparison, version-specific limitations, workarounds
- **[IGNORED_TESTS_ROADMAP.md](IGNORED_TESTS_ROADMAP.md)**: Test tracking and resolution status
- **[Issue #188](https://github.com/EffortlessMetrics/perl-lsp/issues/188)**: Semantic Analyzer (for deeper analysis needs)
- **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)**: LSP feature coverage
- **[PURE_RUST_PARSER.md](../explanation/PURE_RUST_PARSER.md)**: v2 Pest parser architecture

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v0.10.0 | 2025-01 | Comprehensive documentation enhancement, added cross-references |
| v0.8.9 | 2024-12 | Refactored to show resolved issues (PR #261) |
| v0.8.8 | 2024-11 | Initial documentation |

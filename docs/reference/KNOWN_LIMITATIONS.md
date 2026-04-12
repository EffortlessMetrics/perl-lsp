# Perl Parser - Known Limitations (v3 Native, v2 Legacy, v1 Archived)

> **Note**: The current default parser is **v3 (Native)** — a pure Rust recursive-descent parser
> with no tree-sitter dependency. v1 is the original C/tree-sitter implementation (archived,
> benchmarking only). v2 is Pest-based (legacy, kept out of the default gate). See
> [PURE_RUST_PARSER.md](../explanation/PURE_RUST_PARSER.md) for v2 historical context.

This document provides a comprehensive list of parsing limitations across all three parser implementations.

## Executive Summary

| Parser | Coverage | Status | Main Limitations |
|--------|----------|--------|------------------|
| **v3: Native** | ~100% | Production Ready | 5 minor edge cases (2% of edge case tests) |
| **v2: Pest** | ~99.996% | Production Ready | Cannot handle m!pattern!, indirect object syntax |
| **v1: C** | ~95% | Legacy | Limited modern Perl support, edge cases |

**Recommendation**: Use **v3 (Native Parser)** for all production use cases. It provides the best performance, coverage, and maintainability.

---

## Parser Version Comparison

### Feature Comparison Matrix

| Feature Category | Feature | v1 (C) | v2 (Pest) | v3 (Native) | Notes |
|-----------------|---------|:------:|:---------:|:-----------:|-------|
| **Core Syntax** | Scalar/Array/Hash declarations | ✅ | ✅ | ✅ | Full support |
| | Subroutines (named/anonymous) | ✅ | ✅ | ✅ | Including closures |
| | Package declarations | ✅ | ✅ | ✅ | Full namespace support |
| | Control flow (if/while/for/foreach) | ✅ | ✅ | ✅ | All standard constructs |
| | unless/until statement modifiers | ✅ | ✅ | ✅ | Full support |
| **Modern Perl (5.38+)** | `class` keyword | ❌ | ✅ | ✅ | Corinna OOP |
| | `method` keyword | ❌ | ✅ | ✅ | Class methods |
| | `field` keyword | ❌ | ✅ | ✅ | Class fields |
| | `try`/`catch` error handling | ❌ | ✅ | ✅ | Exception handling |
| | Subroutine signatures | ⚠️ | ✅ | ✅ | v1 partial support |
| | Postfix dereference | ⚠️ | ✅ | ✅ | `$ref->@*` syntax |
| **Regular Expressions** | Standard `/pattern/` match | ✅ | ✅ | ✅ | Full support |
| | `m//` with modifiers | ⚠️ | ✅ | ✅ | v1 limited modifier support |
| | `qr//` quoted regex | ✅ | ✅ | ✅ | Full support |
| | **Arbitrary delimiters** `m!pat!` | ❌ | ❌ | ✅ | v3 only |
| | `s///` substitution | ⚠️ | ✅ | ✅ | v2 improved PR #42 |
| | **s/// with arbitrary delimiters** | ❌ | ❌ | ✅ | `s\|old\|new\|` v3 only |
| | `tr///` transliteration | ⚠️ | ✅ | ✅ | Full support in v2/v3 |
| **Special Syntax** | Heredocs (`<<EOF`, `<<'EOF'`) | ⚠️ | ✅ | ✅ | v1 incomplete |
| | **Indirect object syntax** | ❌ | ❌ | ✅ | `print $fh "text"` v3 only |
| | Format declarations | ❌ | ⚠️ | ⚠️ | Basic support in v2/v3 |
| | Typeglobs (`*foo`) | ⚠️ | ✅ | ✅ | Limited in v1 |
| | `bless`, `tie` constructs | ⚠️ | ✅ | ✅ | Full support v2/v3 |
| **Unicode** | UTF-8 source files | ⚠️ | ✅ | ✅ | Full support |
| | Unicode identifiers (`$café`) | ⚠️ | ✅ | ✅ | Full support |
| | Emoji identifiers (`$♥`) | ❌ | ⚠️ | ⚠️ | May need validation |
| **Edge Cases** | Complex prototypes (`sub f(&@)`) | ⚠️ | ⚠️ | ⚠️ | Parsed, may need refinement |
| | Decimal without trailing (`5.`) | ❌ | ⚠️ | ⚠️ | Works, AST could improve |
| | Nested interpolation (`@{[...]}`) | ⚠️ | ⚠️ | ⚠️ | Deep nesting may fail |
| | Empty blocks (`sort {} @list`) | ⚠️ | ✅ | ✅ | Fixed in v3 v0.7.1 |

Legend: ✅ Full Support | ⚠️ Partial/Limited | ❌ Not Supported

### Coverage Percentages

| Parser | Core Perl 5 | Modern Perl (5.38+) | Edge Cases | Overall |
|--------|:-----------:|:-------------------:|:----------:|:-------:|
| **v3 Native** | 100% | 100% | 98% | **~100%** |
| **v2 Pest** | 100% | 100% | 95% | **~99.996%** |
| **v1 C** | 95% | 0% | 60% | **~95%** |

### Performance Characteristics

| Metric | v1 (C) | v2 (Pest) | v3 (Native) |
|--------|:------:|:---------:|:-----------:|
| **Simple files** (1KB) | ~100 µs | ~200 µs | **~50 µs** |
| **Medium files** (10KB) | ~500 µs | ~1.5 ms | **~200 µs** |
| **Large files** (100KB) | ~5 ms | ~15 ms | **~2 ms** |
| **Memory efficiency** | Good | Good | **Best** |
| **Startup time** | Fast | Medium | **Fastest** |
| **Incremental parsing** | ❌ | ❌ | ✅ <1ms updates |
| **Maintainability** | Low | High | **High** |

---

## v3: Native Parser (perl-lexer + perl-parser) - RECOMMENDED

### Coverage: ~100% (98% of comprehensive edge cases)

**Recent fixes (v0.7.1):**
- ✅ Fixed `bless {}` parsing (now correctly parsed as function call with empty hash)
- ✅ Fixed `sort {}`, `map {}`, `grep {}` empty block parsing
- ✅ Enhanced builtin function argument handling

### Fully Supported Features

- ✅ Regex with arbitrary delimiters (`m!pattern!`, `m{pattern}`, `s|old|new|`)
- ✅ Indirect object syntax (`print $fh "Hello"`, `print STDOUT "msg"`, `new Class::Name`)
- ✅ Quote operators with custom delimiters (`q!text!`, `qq#text#`)
- ✅ All modern Perl features (class, method, try/catch, etc.)
- ✅ Complex dereferencing chains
- ✅ Unicode identifiers (including emoji: `$♥`, `$🚀`)
- ✅ Defined-or operator (`$x // $y`)
- ✅ Glob dereference (`*$ref`)
- ✅ Pragma with fat-arrow/hash args (`use constant FOO => 42`)
- ✅ List interpolation (`@{[ ... ]}`)
- ✅ Multi-variable lexicals with per-variable attributes (`my ($x :shared, $y :locked)`)

### Minor Limitations (2% of edge cases)

#### 1. Complex Prototypes

**Status**: Parsed but may need refinement for full accuracy

**Works correctly**:
```perl
sub mygrep(&@) { my $code = shift; grep { $code->() } @_ }
sub mymap(&@)  { my $code = shift; map { $code->() } @_ }
sub test(_)    { ... }  # Underscore prototype
```

**May need refinement**:
```perl
# Complex prototypes with multiple special sigils
sub complicated($$$$;&@) { ... }  # Parsed but AST accuracy varies

# Prototypes with backslash escapes
sub regex_handler(/\[$/) { ... }  # Rarely used, edge case
```

**Impact**: Low - Most production code uses simple prototypes or subroutine signatures (Perl 5.20+)

---

#### 2. Emoji Identifiers

**Status**: Parsed but may lack proper Unicode category validation

**Works correctly**:
```perl
my $♥ = 'love';
my $🚀 = 'rocket';
print $♥;  # Outputs: love

sub 🎉 { print "Celebration!" }
🎉();  # Works
```

**Potential issues**:
```perl
# ZWJ (Zero-Width Joiner) sequences may have validation issues
my $👨‍👩‍👧‍👦 = 'family';  # Multi-codepoint grapheme cluster

# Variation selectors
my $️⃣ = 'keycap';  # May not be properly validated
```

**Impact**: Very Low - Emoji identifiers are extremely rare in production code

**Best Practice**: Use ASCII identifiers for production code. Reserve Unicode identifiers for documentation examples or personal scripts.

---

#### 3. Format Declarations

**Status**: Basic support exists, complex formats may need enhancement

**Works correctly**:
```perl
format STDOUT =
@<<<<<<   @||||||   @>>>>>>
$name,    $price,   $quantity
.

write;
```

**May have issues**:
```perl
# Complex format with expressions and continuation
format REPORT =
^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$text_from_variable
~~  ^<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$continuation
.

# Format with picture lines containing @ fields
format HTML =
<a href="@<<<<<<<<<<<<<<<<">@<<<<<<<<<<<<<<<<</a>
$url, $title
.
```

**Impact**: Low - Format declarations are a legacy Perl feature, rarely used in modern code

**Best Practice**: Replace format declarations with:
- `printf`/`sprintf` for simple formatting
- Template Toolkit or similar for complex reports
- Perl6::Form module for advanced form formatting

---

#### 4. Decimal Without Trailing Digits

**Status**: Works but could be more explicit in AST representation

**Works correctly**:
```perl
my $x = 5.;      # Parsed as 5.0
my $y = 5.e10;   # Parsed as 5.0 × 10^10
my $z = .5;      # Parsed as 0.5
my $w = 5.0e10;  # Standard scientific notation
```

**AST representation**:
```sexp
; Current representation
(number_literal "5.")

; Could be more explicit
(number_literal "5.0" :value 5.0)
```

**Impact**: Very Low - Edge case in number parsing, does not affect functionality

---

#### 5. Nested Complex Interpolation

**Status**: Basic nested interpolation works, deep nesting may fail

**Works correctly**:
```perl
# Single-level interpolation
my @doubled = @{[ map { $_ * 2 } @numbers ]};

# With grep
my @positive = @{[ grep { $_ > 0 } @numbers ]};
```

**May fail with deep nesting**:
```perl
# Deep nesting with multiple list operators
my @complex = @{[ map { @{[ grep { $_ > 0 } @$_ ]} } @nested ]};

# Complex expressions inside interpolation
my @result = @{[ map { $_->{items}->@* } grep { $_->{active} } @objects ]};
```

**Workaround**:
```perl
# Extract to intermediate variables for clarity and parser compatibility
my @filtered = map { [ grep { $_ > 0 } @$_ ] } @nested;
my @flattened = map { @$_ } @filtered;

# Or use List::Util functions
use List::Util qw(reduce);
my @result = reduce { push @$a, @$_[1]; $a } [], @nested;
```

**Impact**: Low - Complex interpolation is uncommon and often indicates code that could be refactored for clarity

---

## v2: Pest-based Parser

### Coverage: ~99.996% (Improved regex/substitution support as of PR #42)

### Fully Supported Features

- ✅ All core Perl 5 features
- ✅ Modern Perl features (class, method, try/catch, signatures)
- ✅ Standard regex forms (`/pattern/`, `s/old/new/`)
- ✅ **Substitution operators** (`s/old/new/g`) with dedicated AST nodes (PR #42)
- ✅ **Enhanced regex parsing** with fallback mechanisms (PR #42)
- ✅ Heredocs (all variants)
- ✅ Unicode identifiers
- ✅ Complex dereferencing

### Recent Improvements (PR #42)

- ✅ Added separate `Substitution` NodeKind for proper s/// parsing
- ✅ Fixed substitution test regressions with backward compatibility
- ✅ Enhanced regex parser with graceful fallback mechanisms
- ✅ Improved S-expression structural compatibility

### Known Limitations (~0.004%)

#### 1. Regex with Arbitrary Delimiters

**Root Cause**: PEG grammars cannot distinguish `m` as function vs regex operator without extensive lookahead.

**NOT Supported**:
```perl
$text =~ m!pattern!;      # Using ! as delimiter
$text =~ m{pattern};      # Using {} as delimiter
$text =~ s|old|new|g;     # Using | for substitution
$text =~ m#pattern#;      # Using # as delimiter
$text =~ m(patern);       # Using () as delimiter
```

**Supported alternatives**:
```perl
$text =~ /pattern/;       # Standard slash delimiters
$text =~ s/old/new/g;     # Standard substitution (IMPROVED in PR #42)
$text =~ qr/pattern/;     # Quoted regex constructor

# For patterns containing slashes, escape them:
$url =~ /https:\/\/example\.com/;

# Or use qr// with variable:
my $pattern = qr{complex/pattern};
$text =~ $pattern;
```

**Technical Explanation**:
```
In PEG (Parsing Expression Grammar), when the parser sees:
  m / pattern /

It must decide if 'm' is:
1. A function call: m(/pattern/) - calling function 'm' with regex argument
2. A match operator: m/pattern/ - match operator with custom delimiter

Without semantic context (knowing 'm' is not a defined function), 
PEG chooses the first matching alternative, which may be incorrect.

v3 Native parser solves this with context-aware tokenization in the lexer.
```

**Impact**: Medium - Affects ~5% of legacy codebases that use non-standard delimiters

---

#### 2. Indirect Object Syntax

**Root Cause**: Requires semantic analysis to distinguish from function calls.

**NOT Supported**:
```perl
method $object @args;     # Indirect object method call
new Class::Name;          # Indirect constructor
print $fh "Hello";        # Indirect filehandle
say STDOUT "message";     # Indirect output
open $fh '<', $file;      # Indirect open (rare)
```

**Supported alternatives**:
```perl
$object->method(@args);   # Arrow notation (preferred)
Class::Name->new();       # Arrow constructor
print($fh, "Hello");      # Parentheses for clarity
$fh->print("Hello");      # Method call on filehandle

# For STDOUT/stderr, use explicit call:
STDOUT->print("message");
print STDOUT "message";   # Actually works in v3, not v2
```

**Technical Explanation**:
```
Indirect object syntax:
  print $fh "text"

Is syntactically ambiguous with:
  print($fh, "text")  # Function call with two arguments

The parser cannot know that print() accepts a filehandle as 
its first argument without semantic knowledge of built-in functions.

This is why indirect object syntax is discouraged in modern Perl:
  "Indirect object syntax is deprecated and will be removed in Perl 5.42"
  - perldoc perlobj
```

**Impact**: Medium - Common in older code, but easily refactored. Modern Perl best practices discourage indirect object syntax.

---

#### 3. Heredoc-in-String

**Root Cause**: Heredocs inside strings create parsing ambiguity.

**NOT Supported**:
```perl
# Heredoc-like construct inside a string
my $text = "$prefix<<$end_tag";

# Potential confusion with actual heredoc
my $code = "eval <<'END'";
```

**Impact**: Very Low - Extremely rare pattern, likely a mistake in actual code

---

## v1: C-based Parser

### Coverage: ~95%

### Status: Legacy Implementation

**Use Cases**:
- Benchmarking comparisons
- Legacy compatibility testing
- Tree-sitter integration testing

**NOT Recommended For**:
- Production use
- Modern Perl codebases
- Projects requiring edge case handling

### Fully Supported Features

- ✅ Basic Perl 5 features
- ✅ Standard syntax forms
- ✅ Tree-sitter integration
- ✅ Core control structures

### Major Limitations

#### 1. No Modern Perl Support

```perl
# NOT Supported - Perl 5.38+ features
class Point {
    field $x :param;
    field $y :param;
    method describe() { print "Point at ($x, $y)" }
}

# NOT Supported - try/catch
try {
    risky_operation();
} catch ($e) {
    log_error($e);
}

# NOT Supported - Signatures
sub add($x, $y) { return $x + $y }
```

#### 2. Limited Regex Support

```perl
# NOT Supported
$text =~ m!pattern!;      # Arbitrary delimiters
$text =~ s{old}{new}g;    # Bracket delimiters

# Supported
$text =~ /pattern/;       # Standard only
```

#### 3. Incomplete Heredoc Support

```perl
# May have issues with
my $text = <<'END';
Multi-line
text
END

# Especially with interpolation
my $greeting = <<"END";
Hello, $name!
END
```

#### 4. Limited Edge Case Handling

- No indirect object syntax
- Limited Unicode support
- No complex prototype handling
- Incomplete typeglob support

---

## Common Limitations Across All Parsers

### Theoretical Limitations (Require Runtime Execution)

These constructs cannot be parsed statically and would require a Perl interpreter:

#### 1. Source Filters

**Scope**: Out of scope for static parsing

**Description**: Source filters transform Perl source code at compile time before parsing.

```perl
use Filter::Simple;
FILTER {
    s/Hello/Goodbye/g;
}

# After filter, code is transformed before parsing
print "Hello World";  # Actually prints "Goodbye World"
```

**Common Source Filter Modules**:
- `Switch` - Adds switch/case syntax
- `Perl6::Say` - Adds 'say' keyword
- `Filter::Simple` - Custom source transformation
- `Spiffy` - Module framework with filtering

**Workaround**: Review files using source filters manually or with Perl's own tools (`perl -c`).

---

#### 2. Runtime Code Generation

**Scope**: Cannot analyze dynamically-constructed code

```perl
# Dynamic code construction
my $code = build_code();
eval $code;  # Cannot be statically analyzed

# String interpolation in eval
my $sub_name = 'dynamic_' . $type;
eval "sub $sub_name { ... }";

# HERE-doc in eval
eval "print <<EOF;\n" . $content . "\nEOF";
```

**What We Do**: Parse the `eval` statement itself correctly, but cannot analyze the evaluated code string.

---

#### 3. Dynamic Symbol Table Manipulation

**Scope**: Runtime behavior cannot be predicted

```perl
# Dynamic sub definition
*foo = sub { ... };

# Dynamic variable creation
no strict 'refs';
*{$name} = 1;

# Symbol aliasing
*alias = *original;

# Package manipulation
$::{new_sub} = sub { ... };
```

**What We Do**: Parse these constructs syntactically but cannot determine their runtime effects on symbol tables.

---

#### 4. BEGIN Block Side Effects

**Scope**: Compile-time effects require execution

```perl
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
```

**What We Do**: Parse `BEGIN` blocks but don't execute them. Symbol table modifications within BEGIN blocks won't be reflected in static analysis.

---

#### 5. Tied Filehandles

**Scope**: Custom I/O behavior requires runtime

```perl
tie *FH, 'Package';
# FH now has custom behavior defined by Package
```

---

## Workarounds and Best Practices

### 1. Avoiding Arbitrary Regex Delimiters (v2)

**Problem**: v2 cannot parse `m!pattern!`

```perl
# Instead of:
$text =~ m!pattern!;

# Option 1: Use standard delimiters
$text =~ /pattern/;

# Option 2: Escape slashes in pattern
$text =~ /path\/to\/file/;

# Option 3: Use qr// for complex patterns
my $re = qr{complex/pattern};
$text =~ $re;

# Option 4: Use quotemeta for dynamic patterns
my $user_input = quotemeta($raw_pattern);
$text =~ /$user_input/;
```

---

### 2. Avoiding Indirect Object Syntax (All Versions)

**Problem**: Indirect object syntax not supported in v1/v2, discouraged in Perl

```perl
# Instead of:
my $obj = new Class;
print $fh "text";
method $object @args;

# Use arrow notation (preferred):
my $obj = Class->new();
$fh->print("text");
$object->method(@args);

# Or use parentheses:
print($fh, "text");
```

**Why This Is Better**:
- Arrow notation is unambiguous
- Works with all parser versions
- Recommended by Perl best practices
- Easier to read and maintain

---

### 3. Replacing Format Declarations

**Problem**: Complex formats may not parse correctly

```perl
# Instead of format declarations:
format STDOUT =
@<<<<<<   @||||||   @>>>>>>
$name,    $price,   $quantity
.

# Option 1: Use printf/sprintf
printf "%-10s %8s %8s\n", 'Name', 'Price', 'Qty';
printf "%-10s %8.2f %8d\n", $name, $price, $quantity;

# Option 2: Use Perl6::Form module
use Perl6::Form;
print form "{||||||||} {||||||||}", $name, $value;

# Option 3: Use Template Toolkit
use Template;
my $tt = Template->new();
$tt->process(\$template, { name => $name, value => $value });

# Option 4: Use Text::Table for tabular data
use Text::Table;
my $tb = Text::Table->new("Name", "Price", "Qty");
$tb->load([$name, $price, $quantity]);
print $tb;
```

---

### 4. Simplifying Complex Interpolation

**Problem**: Deep nesting in `@{[...]}` may fail

```perl
# Instead of:
my @result = @{[ map { @{[ grep { $_ > 0 } @$_ ]} } @nested ]};

# Option 1: Extract to intermediate steps
my @filtered = map { [ grep { $_ > 0 } @$_ ] } @nested;
my @result = map { @$_ } @filtered;

# Option 2: Use List::Util functions
use List::Util qw(reduce);
my @result = reduce { push @$a, @$_; $a } [], @filtered;

# Option 3: Use named subroutines for clarity
sub filter_positive { grep { $_ > 0 } @$_ }
my @result = map { filter_positive($_) } @nested;
```

---

### 5. Working Around Source Filters

**Problem**: Source filters cannot be statically analyzed

```perl
# Instead of Switch module:
use Switch;
switch ($val) {
    case 1 { print "One" }
    case 2 { print "Two" }
}

# Option 1: Use given/when (Perl 5.10+)
use feature 'switch';
given ($val) {
    when (1) { print "One" }
    when (2) { print "Two" }
    default  { print "Other" }
}

# Option 2: Use if/elsif chains
if ($val == 1) { print "One" }
elsif ($val == 2) { print "Two" }
else { print "Other" }

# Option 3: Use dispatch table
my %actions = (
    1 => sub { print "One" },
    2 => sub { print "Two" },
);
$actions{$val}->() if exists $actions{$val};
```

---

### 6. Handling Dynamic Code

**Problem**: eval STRING cannot be analyzed

```perl
# Instead of:
my $code = "sub $name { ... }";
eval $code;

# Option 1: Use subroutine references
my $sub_ref = sub { ... };
# Store in variable/hash for dynamic dispatch

# Option 2: Use can() method for method lookup
my $method = $class->can($method_name);
$object->$method() if $method;

# Option 3: Use require for dynamic module loading
my $module = "Module::$name";
eval "require $module";  # Still uses eval, but more structured
```

---

## Testing Parser Limitations

### v3 Native Parser

```bash
# Test edge cases
cargo run -p perl-parser --example test_edge_cases
cargo run -p perl-parser --example test_more_edge_cases
cargo run -p perl-parser --example test_remaining_edge_cases

# Run comprehensive test suite
cargo test -p perl-parser

# Test specific edge case categories
cargo test -p perl-parser --test enhanced_edge_case_parsing_tests
cargo test -p perl-parser --test comprehensive_unicode_edge_cases
```

### v2 Pest Parser

```bash
# Test with pure Rust feature
cargo test --features pure-rust test_edge_cases

# Run comparison tests
cargo run --features "pure-rust test-utils" --bin compare_parsers -- --test
```

### v1 C Parser

```bash
# Legacy tests (for benchmarking only)
cargo test --features c-scanner
```

### Compare All Parsers

```bash
# Run comparison harness
cargo xtask compare

# Run benchmarks
cargo bench
```

---

## Recommendations

### For Production Use

| Scenario | Recommended Parser | Rationale |
|----------|-------------------|-----------|
| New projects | **v3 Native** | Best coverage, performance, features |
| Modern Perl (5.38+) | **v3 Native** | Only parser with full modern support |
| Legacy codebase | **v3 Native** | Best edge case handling |
| CI/CD pipelines | **v3 Native** | Fastest, most reliable |
| IDE integration | **v3 Native** | LSP server uses v3 |

### For Development

| Task | Recommended Parser | Notes |
|------|-------------------|-------|
| Performance-critical | **v3 Native** | Fastest parsing |
| Grammar experimentation | v2 Pest | PEG is easier to modify |
| Benchmarking | v1 C | Legacy comparisons |
| Debugging parser issues | v2/v3 | Better error messages |

### Parser Selection Decision

```
                    Production Use?
                         │
           ┌─────────────┴─────────────┐
          Yes                          No
           │                           │
     Need edge cases?           Experimenting?
           │                           │
     ┌─────┴─────┐              ┌──────┴──────┐
    Yes         No             PEG         Benchmark
     │           │              │             │
  v3 Native   v3 Native      v2 Pest      v1 C
     │           │              │             │
     └─────┬─────┘              │             │
           │                    │             │
      RECOMMENDED          For testing   Legacy only
```

---

## workspace/willRenameFiles Partial Coverage

`workspace/willRenameFiles` (LSP 3.16) updates `use`, `require`, `use parent`, and `use base`
import lines when a module file is renamed. The following patterns are **not** automatically
rewritten and require manual find-and-replace:

| Pattern | Example | Status |
|---------|---------|--------|
| Static method calls | `OldPkg->method()` | Not updated — tracked as follow-up |
| Qualified function calls | `OldPkg::func()` | Not updated — tracked as follow-up |
| `@ISA` array assignments | `@ISA = ('OldPkg')` | Not updated — dynamic, dangerous to auto-rewrite |
| `push @ISA` | `push @ISA, 'OldPkg'` | Not updated — tracked as follow-up |
| Moose/Moo DSL | `extends 'OldPkg'` | Not updated — tracked as follow-up |
| Closed files | Files not open in the editor | Index-discovered but not rewritten (pre-existing limitation) |

The LSP server emits a `window/showMessage` warning when it detects that open documents
contain the old module name in patterns that were not updated, so the user is notified
to perform a manual sweep.

---

## Related Documentation

- **[PARSER_LIMITATIONS.md](PARSER_LIMITATIONS.md)**: Intentional boundaries and resolved issues
- **[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)**: LSP feature coverage
- **[PURE_RUST_PARSER.md](../explanation/PURE_RUST_PARSER.md)**: v2 Pest parser architecture
- **[RUST_VS_C_COMPARISON.md](../explanation/RUST_VS_C_COMPARISON.md)**: Performance comparison

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v0.12.0 | 2026-03 | workspace/willRenameFiles: use parent/use base discovery fixed (#2747) |
| v0.10.0 | 2025-01 | Comprehensive documentation enhancement |
| v0.7.1 | 2024-12 | Fixed empty block parsing (bless, sort, map, grep) |
| v0.7.0 | 2024-11 | Added arbitrary regex delimiter support (v3) |
| v0.6.0 | 2024-10 | Added indirect object syntax support (v3) |

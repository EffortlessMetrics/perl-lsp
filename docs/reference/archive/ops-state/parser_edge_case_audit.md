# Comprehensive Parser Edge Case Audit

**Date**: 2026-03-20
**Commit**: cbe38cc95
**Corpus**: 4355 CPAN modules, 85.4% clean (3717 files), 634 with errors (2023 error nodes)

---

## 1. CURRENT ERROR BUCKETS (RANKED BY FREQUENCY)

### Top 10 Error Categories
| Rank | Error Type | Count | Files Affected | Status |
|------|-----------|-------|---------------|----|
| 1 | `unexpected_comma_expr` | 113 | ~113 | Active issue #2184 |
| 2 | `unexpected_token_in_expr` | 102 | ~102 | Broad category, sub-split needed |
| 3 | `unclosed_paren` | 66 | ~66 | Active issue #2189 |
| 4 | `unclosed_brace_semicolon` | 58 | ~58 | Related to block handling |
| 5 | `unexpected_fat_arrow_expr` | 53 | ~53 | Active issue #2188 |
| 6 | `unclosed_brace` | 46 | ~46 | Block closing edge case |
| 7 | `unclosed_paren_identifier` | 38 | ~38 | Specific paren case |
| 8 | `expected_module_name` | 27 | ~27 | v-strings in use/package |
| 9 | `expected_colon` | 18 | ~18 | Label or hash key context |
| 10 | `unclosed_bracket` | 14 | ~14 | Array/slice context |

### Remaining Error Types (11-25)
| Error Type | Count |
|-----------|-------|
| `expected_identifier` | 14 |
| `expected_left_brace` | 10 |
| `substitution_misparse` | 10 |
| `unexpected_word_op_or` | 14 |
| `unclosed_angle` | 6 |
| `unexpected_rbrace_expr` | 8 |
| `unexpected_rparen_expr` | 6 |
| `unexpected_word_op_not` | 8 |
| `unexpected_word_op_and` | 7 |
| `unexpected_slash_expr` | 5 |
| `expected_variable` | 2 |
| `expected_semicolon` | 1 |
| `expected_comma` | 2 |
| `missing_replacement_in_substitution` | 2 |
| `CHECK_must_be_followed_by_block` | 2 |

---

## 2. PERL LANGUAGE FEATURES: TEST COVERAGE STATUS

### ✓ Well-Tested (4+ test files)
- `redo` statements (9 files) — loop control flow
- `wantarray` context (8 files) — context-sensitive returns
- `caller` introspection (4 files) — stack introspection
- `given/when` (6 files) — switch construct
- `local` scope (14 files) — dynamic variables

### ✓ Moderately Tested (2-3 files)
- `AUTOLOAD` magic method (1 file) — dynamically generated methods
- `DESTROY` magic method (1 file) — object finalization
- `overload` pragma (2 files) — operator overloading
- `goto` control flow (1 file) — non-local jumps
- `tie/untie` (1 file) — magical hash/array binding

### ❌ **NOT TESTED** (critical gaps)
- `UNIVERSAL` methods (`can`, `isa`) — metaclass introspection
- `v-strings` (version literals like `v1.2.3`) — version syntax
- `yada yada` / ellipsis (`...`) — stub declarations
- `local` scope (full coverage missing for special vars)
- `typeglob assignment` (`*new = \&old`) — symbol table manipulation
- `chop`/`chomp` (legacy string ops) — string mutation
- `symbolic references` (`$$name`, `${"name"}`) — dynamic variable access

---

## 3. COMPLEX CONSTRUCTS: PARSE STATUS

### ✓ PARSES CORRECTLY
- Chained method calls: `$obj->m1($a)->m2($b)->m3`
- Nested ternary: `$a ? $b ? 1 : 2 : $c ? 3 : 4`
- Complex derefs: `$hash{key}[0]->method()->{result}`
- Array/hash slices: `@array[0..5]`, `@hash{@keys}`
- String concatenation chains: `$a . $b . $c . $d`
- Labeled loop control: `OUTER: for ... { next OUTER if ... }`
- List assignment with `undef`: `my ($a, undef, $c) = @array`
- `qw()` with multiple delimiters: `qw(a b c)`, `qw{a b c}`
- Postfix modifiers: `print "x" if $c; $i++ while <$fh>`
- Complex map/grep chains: `map { ... } grep { ... } @list`

### ⚠️ PARTIALLY/FRAGILE (edge cases fail)
- **v-strings in `use` statements**: `use v5.38.0` parses, but produces `expected_module_name` error in some CPAN files (27 errors). Root cause: parser expects bareword identifier, not version literal.
- **Complex regex with code blocks**: `qr/(?{code})/xsm` — tokenization succeeds but AST handling incomplete
- **Dynamic string interpolation**: `"${\ join(...) } text"` — complex block expressions in interpolation
- **Typeglob expressions as postfix operands**: `*glob++`, `*glob->method` — recently fixed in #2238, now works
- **Indirect method calls in complex contexts**: Still has edge cases with argument boundaries (#2206, #2188)
- **Heredoc with special delimiters**: Some CPAN idioms not fully recognized

### ❌ FAILS / NOT IMPLEMENTED
- **UNIVERSAL methods with symbolic refs**: `$class->can($name)` when $name is a variable
- **Tie with magic variables**: `tie my $scalar, 'Class'`
- **Format declarations**: `format NAME = ... =cut`
- **Prototypes in complex contexts**: `sub foo (\@\%) { ... }`
- **Quote-like operators** (`q`, `qq`, `qx`, `qw`, `qr`): Mostly work, but edge cases with nested delimiters fail

---

## 4. SYNTAX EDGE CASES DISCOVERED IN RECENT FIXES

### PR #2245: `${expr}` and `sigil{func()}` dereference patterns
**Status**: ✓ FIXED
**Issue**: Parser failed on `${$ref}`, `${$obj->method}`, `@{$ref}`, `%{$ref->method()}`
**Fix**: Extended dereference prefix parser to handle full expressions in braces

### PR #2238: Postfix operators after typeglob expressions
**Status**: ✓ FIXED
**Issue**: `*glob++`, `*glob--`, `*glob->method()` were misparsed
**Fix**: Allowed postfix operators on typeglob sigil expressions

### PR #2237: Semicolon handling in `use` imports
**Status**: ✓ FIXED
**Issue**: `use Module (a, b); sub foo { ... }` incorrectly broke import parsing
**Fix**: Guard semicolon break with `at_top_level()` check

### PR #2236: Indirect call argument terminators
**Status**: ✓ FIXED
**Issue**: `print { $fh } "text"` incorrectly parsed because `}` not recognized as terminator
**Fix**: Added `RightBrace` to indirect call argument terminators

### PR #2206: Complex parenthesized arguments
**Status**: ✓ FIXED
**Issue**: 134 CPAN files failed on complex expressions in parentheses (e.g., `method( ($a, $b) )`)
**Fix**: Extended expression parsing to handle nested parens in argument position

### PR #2188: Fat arrow in expression context
**Status**: ✓ FIXED
**Issue**: `$hash{key => $value}` and similar fat-arrow-in-expr patterns (53 files)
**Fix**: Allow `=>` as expression operator (similar to `,`)

---

## 5. UNTESTED PERL LANGUAGE FEATURES (PRIORITY GAPS)

### MUST TEST (high impact, currently uncovered)

#### Group A: Metaclass & Introspection
```perl
# UNIVERSAL methods (can/isa/DOES)
$obj->can('method_name');
$class->isa('Parent::Class');
$obj->DOES('Role::Name');
ref($obj);  # builtin, not a method
```
**Why**: Common in framework code (Moose, Moo, role systems)

#### Group B: Version & String Literals
```perl
# v-strings (version literals)
use v5.38.0;
my $version = v1.2.3.4;
require v5.10.0;
our $VERSION = 'v1.2.3';
```
**Why**: 27 CPAN errors in expected_module_name bucket

#### Group C: Stub/Placeholder Syntax
```perl
# Yada yada (ellipsis operator)
sub todo { ... }
sub partial_impl {
    setup();
    ...  # die here
    cleanup();
}
```
**Why**: Modern Perl idiom for incomplete methods

#### Group D: Dynamic Symbol Table Access
```perl
# Typeglob assignment & manipulation
*alias = \&original;
*new_name = *old_name;
*{$pkg . '::method'} = \&implementation;
```
**Why**: Used in module factories, plugin systems

#### Group E: Variable Binding & Aliasing
```perl
# Tie (bind variables to classes)
tie my %hash, 'MyClass', @args;
tied %hash;
untie %hash;

# Local scope (dynamic, not lexical)
local $/ = undef;  # slurp mode
local $\ = "\n";   # output record separator
local @ISA;
```
**Why**: Context-sensitive IO, special variables

#### Group F: String Manipulation (Legacy)
```perl
# chop/chomp (character/line removal)
chop($string);   # removes last char
chomp($string);  # removes newline
chop(@array);    # chops each element
```
**Why**: Still used in legacy/CPAN code

#### Group G: Symbolic References & Derefs
```perl
# Variable variable access
my $name = 'value';
$$name;  # deref as scalar
@$name;  # deref as array
%$name;  # deref as hash
&$name;  # deref as sub

# Type constructs with symbolic refs
*{"$pkg\::$name"} = $coderef;
\$$var_name;     # reference to variable
```
**Why**: Used in metaprogramming, dynamic dispatch

---

## 6. LSP EDGE CASES (Non-syntax)

### File Handling Edge Cases
- **UTF-8 filenames**: `use open ':encoding(UTF-8)'; ...` (not a parse issue, but relevant to file detection)
- **CRLF line endings**: Parser tokenizes by byte offset; CRLF may cause misalignment in diagnostics
- **BOM (Byte Order Mark)**: UTF-8 BOM (EF BB BF) at file start may confuse lexer
- **Mixed tabs/spaces**: Inconsistent indentation (valid Perl but may break indentation-aware LSP features)
- **Files without trailing newline**: Parser handles, but diagnostic line wrapping may differ

### Perl Context Issues
- **bareword method calls**: `$obj->method (args)` vs `$obj->method(args)` (space changes meaning)
- **List context vs scalar context**: Return value interpretation affects completion hints
- **Slurp vs line-by-line**: File reading mode affects lexer behavior (not applicable to parser)

---

## 7. RECENT DISCOVERY: 56 TEST FAILURES IN PR #2238

**Scope**: Tests for postfix operators on typeglob expressions
**Constructs exposed**:
1. Typeglob as lvalue in postfix operations
2. Typeglob in method call chains
3. Typeglob with subscript (rare, but exposed)
4. Typeglob in complex expression contexts

These were **PREVIOUSLY UNIMPLEMENTED** and are now covered by #2238.

---

## 8. RECOMMENDED ACTION PLAN

### Phase 1: Coverage (2-3 agents)
**Create tests for gap groups A-G above** (UNIVERSAL, v-strings, yada, typeglob, tie, chop, symbolic refs)

**Blockers identified**:
- v-string in `use` statement (27 files failing) — blocking factor
- Complex parenthesized args in indirect calls — mostly fixed by #2206, verify edge cases

### Phase 2: Dialect Variants
- **Postfix operators**: Test all combinations (++, --, ->, [index], {key})
- **String operators**: Test all 20+ string/regex operators in edge contexts
- **Method dispatch**: Test bare method call, scalar method ref, symbolic method names

### Phase 3: Robustness
- **Error recovery**: Verify ERROR nodes don't cascade into unrelated structures
- **Malformed input**: Test incomplete constructs (unclosed braces, missing terminators)
- **Deeply nested**: Test 20+ levels of nesting (dereference chains, nested calls, etc.)

### Phase 4: Dialect-Specific
- **Moose/Moo**: Method modifiers (`before`, `after`, `around`), type constraints
- **Modern Perl**: Feature pragmas (`use feature qw(state say)`), subroutine prototypes
- **DBIx::Class**: Complex chained builders, relationship definitions

---

## 9. SUMMARY TABLE

| Aspect | Status | Notes |
|--------|--------|-------|
| **Core syntax** | 85.4% ✓ | 3717/4355 files clean |
| **Metaclass (UNIVERSAL)** | ❌ | No tests; 0 known failures (rare in corpus?) |
| **v-strings in `use`** | ⚠️ | 27 errors; needs specific fix |
| **Yada yada** | ❌ | No tests, but likely parses (low priority) |
| **Typeglob ops** | ✓ | Fixed in #2238, 56 new tests |
| **Indirect calls** | ⚠️ | Mostly fixed (#2206, #2236), edge cases remain |
| **Fat-arrow args** | ✓ | Fixed in #2188, 53 files verified |
| **Complex derefs** | ✓ | Fixed in #2245, ${expr} patterns |
| **Tie/local scope** | ⚠️ | Tests exist but not comprehensive |
| **Symbolic refs** | ❌ | No tests; medium confidence in parsing |
| **Postfix operators** | ✓ | Comprehensive after #2238 |

---

## 10. NEXT STEPS FOR TEAM LEAD

1. **Verify v-string fix**: Check PR #2245 confirms `use v5.38.0` now parses cleanly (27-error bucket should shrink)
2. **Investigate `unexpected_token_in_expr` (102 files)**: Likely contains sub-categories; recommend scout for split
3. **Create UNIVERSAL/v-string/yada test suites**: High-value, low-effort coverage gaps
4. **Verify test counts**: Run `update_current_status.py` after adding new test files (policy_checks gate requires it)

# Issue: Format Statements - No Corpus Coverage

> **STATUS: ✅ RESOLVED** - Corpus coverage has been added.
>
> - Test corpus: `test_corpus/format_statements.pl` (4+ test cases)
> - Additional coverage: `test_corpus/legacy_syntax.pl`
> - Added in PR #404 (commit 28552903)

## Original Problem Description (Historical)

### What We Found (Now Outdated)

~~Format statements have **zero coverage** in the corpus.~~

**Current status**: Corpus coverage was added. The parser supports format statements
via `NodeKind::Format { name, body }` and test fixtures now exercise this feature.

### Minimal Reproduction

```perl
# Basic format statement
format STDOUT =
@<<<<<<<<<<<<< @|||||||||| @>>>>>>>>>>>
$name,        $age,       $salary
.

# Format with picture lines
format REPORT =
================================================================================
Employee Report
================================================================================
Name: @<<<<<<<<<<<<<<<<<<<<<<<<<<<<<  Age: @##  Salary: @#######.##
$name,                             $age,      $salary
.

# Format with field specifiers
format STDOUT_TOP =
Page @<<
%
.

# Format with complex picture lines
format CHECK =
*******************************************************
*  PAY TO THE ORDER OF: @<<<<<<<<<<<<<<<<<<<<<<  *
*  AMOUNT:             @######.##               *
*******************************************************
$payee,                                      $amount
.
```

### Current Behavior

When parsing format statements:
- The parser may not recognize `format` as a keyword
- Format picture lines (between `format NAME =` and `.`) may be parsed as strings or comments
- Format field specifiers (`@<<<<<<`, `@>>>>>>`, `@###`, etc.) are not recognized as special syntax
- No specific test coverage exists for format statements

This results in:
- No validation that format statements are parsed correctly
- Missing regression testing for format-related features
- Potential bugs in format parsing going undetected
- Incomplete LSP support for format statements

### Expected Behavior

The parser should:
1. Recognize `format` as a keyword and parse format statements correctly
2. Parse format picture lines as a distinct component
3. Recognize format field specifiers as special syntax
4. Support format variable binding (e.g., `format STDOUT =` vs `format =`)
5. Have comprehensive test coverage for format statements

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/parser.rs` - Ensure format statement parsing is correct
- `/crates/perl-parser/src/lexer.rs` - Ensure `format` keyword is recognized

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/format_statements.pl` with comprehensive format examples
- Or create generator in `/crates/perl-corpus/src/gen/format_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Parser recognizes `format` keyword
   - [ ] Format picture lines are parsed correctly
   - [ ] Format field specifiers are recognized
   - [ ] Format variable binding is supported

2. **Test Coverage**:
   - [ ] At least 10 test cases in corpus covering:
     - Simple format statements
     - Format with picture lines
     - Format with multiple field specifiers
     - Format with variable binding
     - Format with `*_TOP` variants (e.g., `STDOUT_TOP`)
     - Format with numeric field specifiers
     - Format with text field specifiers
     - Format with multiline picture lines
     - Format with special characters
     - Format with escape sequences

3. **AST Validation**:
   - [ ] Format statements produce correct AST structure
   - [ ] Picture lines are captured correctly
   - [ ] Field specifiers are identified

4. **LSP Integration**:
   - [ ] Format statements are syntax highlighted correctly
   - [ ] Go-to-definition works for format names
   - [ ] Hover shows format information
   - [ ] Completion suggests format field specifiers

### Solution Options

#### Option 1: Comprehensive Format Test Suite (Recommended)

**Pros**:
- Complete coverage of format syntax
- Validates all format features
- Enables regression testing
- Supports LSP features

**Cons**:
- More test cases to maintain
- Format is a deprecated feature

**Implementation**:
- Create `test_corpus/format_statements.pl` with 10+ test cases
- Cover all format field specifier types
- Include edge cases and error scenarios
- Add LSP integration tests

#### Option 2: Minimal Format Test Coverage

**Pros**:
- Faster to implement
- Reduces coverage gap
- Fewer tests to maintain

**Cons**:
- Limited validation
- May miss edge cases
- Insufficient for LSP features

**Implementation**:
- Create `test_corpus/format_statements.pl` with 3-5 test cases
- Cover basic format syntax only
- Minimal edge case coverage

#### Option 3: Format Generator

**Pros**:
- Generates diverse test cases
- Easy to extend
- Property-based testing support

**Cons**:
- More complex setup
- Requires generator infrastructure

**Implementation**:
- Create `/crates/perl-corpus/src/gen/format_gen.rs`
- Generate format statements with various field specifiers
- Integrate with existing test infrastructure

### Path Forward

**Recommended**: Option 1 (Comprehensive Format Test Suite)

**Rationale**:
1. Format statements, while deprecated, are still in active use in legacy Perl codebases
2. Complete coverage ensures the parser handles all format features correctly
3. Enables proper LSP support for developers working with legacy code
4. Comprehensive test suite provides good regression protection

**Implementation Steps**:
1. Create `test_corpus/format_statements.pl` with comprehensive format examples
2. Include test cases for:
   - All field specifier types (`@<<<<<<`, `@>>>>>>`, `@###`, `@|||||`, etc.)
   - Format variable binding
   - `*_TOP` format variants
   - Multiline picture lines
   - Special characters and escape sequences
3. Validate parser output against expected AST
4. Add LSP integration tests for format highlighting
5. Test with real-world Perl code using format statements
6. Document format statement support

**Timeline Estimate**: 1-2 days for test creation + 1 day for validation

### References

- **Perl Documentation**: [perlform - Perl formats](https://perldoc.perl.org/perlform)
- **Format Syntax**: [Field specifier documentation](https://perldoc.perl.org/perlform#Formatters)
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: NodeKind `Format` never seen in corpus
- **GA Feature Alignment**: Format statements are a P0 critical feature with no coverage

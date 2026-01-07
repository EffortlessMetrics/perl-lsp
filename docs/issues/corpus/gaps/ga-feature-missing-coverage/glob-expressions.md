# Issue: Glob Expressions - No Corpus Coverage

## Problem Description

### What We Found

Glob expressions have **zero coverage** in the corpus despite being a P0 critical feature in the GA (Grammar Analyzer) feature list:
- Tree-sitter corpus (`tree-sitter-perl/test/corpus/`): 0 test cases
- Highlight fixtures (`tree-sitter-perl/test/highlight/`): 0 test cases
- Test corpus (`test_corpus/`): 0 test cases
- Perl-corpus generators (`crates/perl-corpus/src/gen/`): 0 generators

This represents a critical gap in test coverage for file pattern matching, a commonly used Perl feature.

### Minimal Reproduction

```perl
# Basic glob operator
my @files = glob "*.pl";

# Glob with angle brackets (equivalent syntax)
my @files = <*.pl>;

# Glob with path patterns
my @all_pl = glob "**/*.pl";
my @hidden = glob ".*";

# Glob in scalar context (iterates)
while (my $file = glob "*.txt") {
    print "Found: $file\n";
}

# Glob with complex patterns
my @matches = glob "/tmp/{a,b,c}*.txt";
my @nested = glob "dir1/*/dir2/*.pm";

# Glob with character classes
my @files = glob "[a-z]*.pl";

# Glob with brace expansion
my @files = glob "file{1,2,3}.txt";

# Glob with recursive patterns
my @all = glob "**/*.pm";
```

### Current Behavior

When parsing glob expressions:
- The `glob` keyword may be parsed as a bareword function call
- Angle bracket glob syntax `<*.pl>` is parsed as readline operator or generic filehandle
- No specific test coverage exists for glob expressions
- Glob patterns are treated as string literals or unknown syntax

This results in:
- No validation that glob expressions are parsed correctly
- Missing regression testing for glob-related features
- Potential bugs in glob parsing going undetected
- Incomplete LSP support for glob pattern validation

### Expected Behavior

The parser should:
1. Recognize `glob` as a builtin operator
2. Parse glob patterns correctly (both `glob()` and angle bracket syntax)
3. Support glob pattern syntax (wildcards, character classes, brace expansion)
4. Distinguish glob from similar constructs (readline, filehandle operations)
5. Have comprehensive test coverage for glob expressions

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/parser.rs` - Ensure glob expression parsing is correct
- `/crates/perl-parser/src/lexer.rs` - Ensure glob patterns are tokenized correctly

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/glob_expressions.pl` with comprehensive glob examples
- Or create generator in `/crates/perl-corpus/src/gen/glob_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Parser recognizes `glob()` function syntax
   - [ ] Parser recognizes angle bracket glob syntax `<pattern>`
   - [ ] Glob patterns are parsed correctly
   - [ ] All glob pattern types are supported

2. **Test Coverage**:
   - [ ] At least 10 test cases in corpus covering:
     - Simple glob patterns (`*.pl`, `*.txt`)
     - Recursive glob patterns (`**/*.pm`)
     - Hidden files (`.*`)
     - Character classes (`[a-z]*`)
     - Brace expansion (`{a,b,c}*`)
     - Nested path patterns
     - Scalar context iteration
     - List context expansion
     - Complex patterns
     - Angle bracket syntax

3. **AST Validation**:
   - [ ] Glob expressions produce correct AST structure
   - [ ] Glob pattern is captured
   - [ ] Context (scalar vs list) is preserved

4. **LSP Integration**:
   - [ ] Glob expressions are syntax highlighted correctly
   - [ ] Hover shows glob pattern information
   - [ ] Completion suggests file patterns
   - [ ] Go-to-definition works for glob-related constructs

### Solution Options

#### Option 1: Comprehensive Glob Test Suite (Recommended)

**Pros**:
- Complete coverage of glob syntax
- Validates all glob features
- Enables regression testing
- Supports LSP features

**Cons**:
- More test cases to maintain

**Implementation**:
- Create `test_corpus/glob_expressions.pl` with 10+ test cases
- Cover all glob pattern types
- Include edge cases and error scenarios
- Add LSP integration tests

#### Option 2: Minimal Glob Test Coverage

**Pros**:
- Faster to implement
- Reduces coverage gap
- Fewer tests to maintain

**Cons**:
- Limited validation
- May miss edge cases
- Insufficient for LSP features

**Implementation**:
- Create `test_corpus/glob_expressions.pl` with 3-5 test cases
- Cover basic glob syntax only
- Minimal edge case coverage

#### Option 3: Glob Generator

**Pros**:
- Generates diverse test cases
- Easy to extend
- Property-based testing support

**Cons**:
- More complex setup
- Requires generator infrastructure

**Implementation**:
- Create `/crates/perl-corpus/src/gen/glob_gen.rs`
- Generate glob expressions with various patterns
- Integrate with existing test infrastructure

### Path Forward

**Recommended**: Option 1 (Comprehensive Glob Test Suite)

**Rationale**:
1. Glob is a commonly used Perl feature for file operations
2. Complete coverage ensures parser handles all glob features correctly
3. Enables proper LSP support for file pattern matching
4. Comprehensive test suite provides good regression protection

**Implementation Steps**:
1. Create `test_corpus/glob_expressions.pl` with comprehensive glob examples
2. Include test cases for:
   - All glob pattern types (wildcards, character classes, brace expansion, recursive)
   - Both `glob()` and angle bracket syntax
   - Scalar and list context
   - Nested path patterns
   - Edge cases and error scenarios
3. Validate parser output against expected AST
4. Add LSP integration tests for glob highlighting
5. Test with real-world Perl code using glob operations
6. Document glob expression support

**Timeline Estimate**: 1-2 days for test creation + 1 day for validation

### References

- **Perl Documentation**: [perlfunc - glob](https://perldoc.perl.org/functions/glob)
- **File::Glob**: [Perl core module for glob operations](https://perldoc.perl.org/File::Glob)
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: NodeKind `Glob` never seen in corpus
- **GA Feature Alignment**: Glob expressions are a P0 critical feature with no coverage

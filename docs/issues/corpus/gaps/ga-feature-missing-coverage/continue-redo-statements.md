# Issue: Continue/Redo Statements - No Corpus Coverage

## Problem Description

### What We Found

Continue/redo statements have **zero coverage** in the corpus despite being a P0 critical feature in the GA (Grammar Analyzer) feature list:
- Tree-sitter corpus (`tree-sitter-perl/test/corpus/`): 0 test cases
- Highlight fixtures (`tree-sitter-perl/test/highlight/`): 0 test cases
- Test corpus (`test_corpus/`): 0 test cases
- Perl-corpus generators (`crates/perl-corpus/src/gen/`): 0 generators

This represents a critical gap in test coverage for loop control statements that are essential for Perl programming.

### Minimal Reproduction

```perl
# Basic continue statement
while (<STDIN>) {
    chomp;
    next if /^#/;      # Skip comments
    last if /^quit/;    # Exit loop
    print "Processing: $_\n";
}

# Continue with label
OUTER: for my $i (0..10) {
    INNER: for my $j (0..10) {
        next OUTER if $i == $j;  # Skip outer iteration
        print "$i, $j\n";
    }
}

# Continue in foreach
foreach my $item (@items) {
    next unless defined $item;  # Skip undefined items
    process_item($item);
}

# Continue in nested loops
while (condition1) {
    while (condition2) {
        continue if $skip;
        # do something
    }
}

# Continue with expression
for my $i (1..100) {
    next if $i % 2 == 0;  # Skip even numbers
    print "$i\n";
}

# Redo statement
while (1) {
    print "Enter a number: ";
    chomp(my $input = <STDIN>);
    last if $input eq 'quit';
    redo unless $input =~ /^\d+$/;  # Redo if not a number
    print "You entered: $input\n";
}

# Redo in foreach
foreach my $file (@files) {
    unless (-e $file) {
        warn "File not found: $file\n";
        redo;  # Try next iteration with same file
    }
    process_file($file);
}

# Redo with label
OUTER: while (condition1) {
    while (condition2) {
        redo OUTER if $retry;
        # do something
    }
}
```

### Current Behavior

When parsing continue/redo statements:
- The parser may not recognize `continue` and `redo` as keywords
- No specific test coverage exists for continue/redo statements
- Continue/redo with labels may not be parsed correctly
- Continue/redo in different loop types may not be handled

This results in:
- No validation that continue/redo statements are parsed correctly
- Missing regression testing for loop control features
- Potential bugs in continue/redo parsing going undetected
- Incomplete LSP support for loop control

### Expected Behavior

The parser should:
1. Recognize `continue` and `redo` as keywords
2. Parse continue/redo statements correctly
3. Support continue/redo with labels
4. Handle continue/redo in different loop types (while, for, foreach, until)
5. Have comprehensive test coverage for continue/redo statements

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/parser.rs` - Ensure continue/redo statement parsing is correct
- `/crates/perl-parser/src/lexer.rs` - Ensure `continue` and `redo` keywords are recognized

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/continue_redo_statements.pl` with comprehensive examples
- Or create generator in `/crates/perl-corpus/src/gen/continue_redo_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Parser recognizes `continue` keyword
   - [ ] Parser recognizes `redo` keyword
   - [ ] Continue/redo statements are parsed correctly
   - [ ] Labels are supported for continue/redo

2. **Test Coverage**:
   - [ ] At least 10 test cases in corpus covering:
     - Continue in while loop
     - Continue in for loop
     - Continue in foreach loop
     - Continue with label
     - Continue with condition
     - Redo in while loop
     - Redo in foreach loop
     - Redo with label
     - Redo with condition
     - Nested continue/redo

3. **AST Validation**:
   - [ ] Continue statements produce correct AST structure
   - [ ] Redo statements produce correct AST structure
   - [ ] Labels are captured correctly
   - [ ] Conditions are captured as children

4. **LSP Integration**:
   - [ ] Continue/redo statements are syntax highlighted correctly
   - [ ] Go-to-definition works for labels
   - [ ] Hover shows continue/redo information
   - [ ] Completion suggests continue/redo keywords

### Solution Options

#### Option 1: Comprehensive Continue/Redo Test Suite (Recommended)

**Pros**:
- Complete coverage of continue/redo statements
- Validates all continue/redo features
- Enables regression testing
- Supports LSP features

**Cons**:
- More test cases to maintain

**Implementation**:
- Create `test_corpus/continue_redo_statements.pl` with 10+ test cases
- Cover all loop types
- Include edge cases and error scenarios
- Add LSP integration tests

#### Option 2: Minimal Continue/Redo Test Coverage

**Pros**:
- Faster to implement
- Reduces coverage gap
- Fewer tests to maintain

**Cons**:
- Limited validation
- May miss edge cases
- Insufficient for LSP features

**Implementation**:
- Create `test_corpus/continue_redo_statements.pl` with 3-5 test cases
- Cover basic continue/redo syntax only
- Minimal edge case coverage

#### Option 3: Continue/Redo Generator

**Pros**:
- Generates diverse test cases
- Easy to extend
- Property-based testing support

**Cons**:
- More complex setup
- Requires generator infrastructure

**Implementation**:
- Create `/crates/perl-corpus/src/gen/continue_redo_gen.rs`
- Generate continue/redo statements with various loop types and conditions
- Integrate with existing test infrastructure

### Path Forward

**Recommended**: Option 1 (Comprehensive Continue/Redo Test Suite)

**Rationale**:
1. Continue/redo are essential Perl loop control statements
2. Complete coverage ensures parser handles all continue/redo features correctly
3. Enables proper LSP support for loop control
4. Comprehensive test suite provides good regression protection

**Implementation Steps**:
1. Create `test_corpus/continue_redo_statements.pl` with comprehensive examples
2. Include test cases for:
   - Continue in all loop types (while, for, foreach, until)
   - Redo in all loop types
   - Continue/redo with labels
   - Continue/redo with conditions
   - Nested continue/redo
   - Edge cases and error scenarios
3. Validate parser output against expected AST
4. Add LSP integration tests for continue/redo highlighting
5. Test with real-world Perl code using continue/redo
6. Document continue/redo statement support

**Timeline Estimate**: 1-2 days for test creation + 1 day for validation

### References

- **Perl Documentation**: [perlsyn - Perl syntax - Loop Control](https://perldoc.perl.org/perlsyn#Loop-Control)
- **Perl Documentation**: [perldoc - continue, redo](https://perldoc.perl.org/functions/continue)
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: NodeKind `Continue`, `Redo` at-risk (<5 occurrences)
- **GA Feature Alignment**: Continue/redo statements are a P0 critical feature with no coverage

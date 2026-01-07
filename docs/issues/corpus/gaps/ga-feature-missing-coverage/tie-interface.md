# Issue: Tie Interface - No Corpus Coverage

## Problem Description

### What We Found

Tie interface has **zero coverage** in the corpus despite being a P0 critical feature in the GA (Grammar Analyzer) feature list:
- Tree-sitter corpus (`tree-sitter-perl/test/corpus/`): 0 test cases
- Highlight fixtures (`tree-sitter-perl/test/highlight/`): 0 test cases
- Test corpus (`test_corpus/`): 0 test cases
- Perl-corpus generators (`crates/perl-corpus/src/gen/`): 0 generators

This represents a critical gap in test coverage for an important Perl feature that enables binding variables to objects.

### Minimal Reproduction

```perl
# Basic tie statement
tie %hash, 'DB_File', 'file.db', O_RDWR|O_CREAT, 0666;

# Tie with array
tie @array, 'Tie::Array';

# Tie with scalar
tie $scalar, 'Tie::Scalar';

# Tie with filehandle
tie *FH, 'Tie::Handle';

# Untie statement
untie %hash;

# Tie with package and arguments
tie %ENV, 'Tie::StdHash';
tie @ISA, 'Tie::Array', @args;

# Tie in object-oriented form
my $obj = tie %hash, 'DBM::Deep', 'file.db';

# Tie with custom package
package MyTie;
require Tie::Hash;
@ISA = qw(Tie::Hash);

sub TIEHASH {
    my ($class, $filename) = @_;
    my $self = {};
    bless $self, $class;
    return $self;
}

sub FETCH {
    my ($self, $key) = @_;
    return $self->{$key};
}

sub STORE {
    my ($self, $key, $value) = @_;
    $self->{$key} = $value;
}

# Usage
tie %myhash, 'MyTie';
```

### Current Behavior

When parsing tie statements:
- The parser may not recognize `tie` as a keyword
- No specific test coverage exists for tie statements
- `untie` statements may not be recognized as a distinct construct
- Tie package and arguments may be parsed as generic function call arguments

This results in:
- No validation that tie statements are parsed correctly
- Missing regression testing for tie-related features
- Potential bugs in tie parsing going undetected
- Incomplete LSP support for tie operations

### Expected Behavior

The parser should:
1. Recognize `tie` as a builtin operator
2. Parse tie statements correctly with variable, package, and arguments
3. Recognize `untie` as a related construct
4. Support all tieable types (scalar, array, hash, filehandle)
5. Have comprehensive test coverage for tie interface

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/parser.rs` - Ensure tie statement parsing is correct
- `/crates/perl-parser/src/lexer.rs` - Ensure `tie` and `untie` keywords are recognized

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/tie_interface.pl` with comprehensive tie examples
- Or create generator in `/crates/perl-corpus/src/gen/tie_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Parser recognizes `tie` keyword
   - [ ] Parser recognizes `untie` keyword
   - [ ] Tie statements are parsed correctly
   - [ ] All tieable types are supported

2. **Test Coverage**:
   - [ ] At least 8 test cases in corpus covering:
     - Tie with hash
     - Tie with array
     - Tie with scalar
     - Tie with filehandle
     - Untie statements
     - Tie with multiple arguments
     - Tie with custom package
     - Tie in object-oriented form

3. **AST Validation**:
   - [ ] Tie statements produce correct AST structure
   - [ ] Variable being tied is captured
   - [ ] Package name is captured
   - [ ] Arguments are captured as children

4. **LSP Integration**:
   - [ ] Tie statements are syntax highlighted correctly
   - [ ] Go-to-definition works for tied package names
   - [ ] Hover shows tie information
   - [ ] Completion suggests tie-related constructs

### Solution Options

#### Option 1: Comprehensive Tie Test Suite (Recommended)

**Pros**:
- Complete coverage of tie interface
- Validates all tie features
- Enables regression testing
- Supports LSP features

**Cons**:
- More test cases to maintain
- Tie is a relatively advanced Perl feature

**Implementation**:
- Create `test_corpus/tie_interface.pl` with 8+ test cases
- Cover all tieable types
- Include edge cases and error scenarios
- Add LSP integration tests

#### Option 2: Minimal Tie Test Coverage

**Pros**:
- Faster to implement
- Reduces coverage gap
- Fewer tests to maintain

**Cons**:
- Limited validation
- May miss edge cases
- Insufficient for LSP features

**Implementation**:
- Create `test_corpus/tie_interface.pl` with 3-5 test cases
- Cover basic tie syntax only
- Minimal edge case coverage

#### Option 3: Tie Generator

**Pros**:
- Generates diverse test cases
- Easy to extend
- Property-based testing support

**Cons**:
- More complex setup
- Requires generator infrastructure

**Implementation**:
- Create `/crates/perl-corpus/src/gen/tie_gen.rs`
- Generate tie statements with various types and packages
- Integrate with existing test infrastructure

### Path Forward

**Recommended**: Option 1 (Comprehensive Tie Test Suite)

**Rationale**:
1. Tie is an important Perl feature for binding variables to objects
2. Complete coverage ensures parser handles all tie features correctly
3. Enables proper LSP support for tie operations
4. Comprehensive test suite provides good regression protection

**Implementation Steps**:
1. Create `test_corpus/tie_interface.pl` with comprehensive tie examples
2. Include test cases for:
   - All tieable types (scalar, array, hash, filehandle)
   - Untie statements
   - Tie with multiple arguments
   - Tie with custom packages
   - Tie in object-oriented form
   - Tie methods (TIEHASH, FETCH, STORE, etc.)
3. Validate parser output against expected AST
4. Add LSP integration tests for tie highlighting
5. Test with real-world Perl code using tie operations
6. Document tie interface support

**Timeline Estimate**: 1-2 days for test creation + 1 day for validation

### References

- **Perl Documentation**: [perltie - How to hide an object class in a simple variable](https://perldoc.perl.org/perltie)
- **Tie::Array, Tie::Hash, Tie::Scalar**: [Core tie modules](https://perldoc.perl.org/Tie::Array)
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: NodeKind `Tie` never seen in corpus
- **GA Feature Alignment**: Tie interface is a P0 critical feature with no coverage

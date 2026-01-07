# Issue: Tie NodeKind Never Seen in Corpus

## Problem Description

### What We Found

The `Tie` NodeKind is **never seen** in any corpus test fixture across all four corpus layers:
- Tree-sitter corpus (`tree-sitter-perl/test/corpus/`): 0 occurrences
- Highlight fixtures (`tree-sitter-perl/test/highlight/`): 0 occurrences
- Test corpus (`test_corpus/`): 0 occurrences
- Perl-corpus generators (`crates/perl-corpus/src/gen/`): 0 generators

This represents a **6% gap** in NodeKind coverage (4 of 68 NodeKinds never seen).

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
```

### Current Behavior

When parsing tie statements:
- The `tie` keyword may be parsed as a bareword function call
- No `Tie` NodeKind is produced in the AST
- The package name and arguments are parsed as generic function call arguments
- `untie` statements are not recognized as a distinct construct

This results in:
- Incomplete AST representation of tie operations
- Missing semantic analysis for tie/untie statements
- Potential incorrect syntax highlighting
- No IDE support for tie-specific features

### Expected Behavior

The parser should:
1. Recognize `tie` as a builtin operator
2. Parse tie statements correctly with variable, package, and arguments
3. Produce a `Tie` NodeKind in the AST
4. Recognize `untie` as a related construct
5. Support all tieable types (scalar, array, hash, filehandle)

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/lib.rs` - Add `Tie` to `NodeKind` enum
- `/crates/perl-parser/src/parser.rs` - Add tie parsing logic
- `/crates/perl-parser/src/lexer.rs` - Ensure `tie` and `untie` keywords are recognized

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/tie_statements.pl` with comprehensive tie examples
- Or create generator in `/crates/perl-corpus/src/gen/tie_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] `Tie` NodeKind exists in `NodeKind` enum
   - [ ] Parser recognizes `tie` keyword
   - [ ] Parser recognizes `untie` keyword
   - [ ] Tie statements are parsed correctly

2. **Test Coverage**:
   - [ ] At least 6 test cases in corpus covering:
     - Tie with hash
     - Tie with array
     - Tie with scalar
     - Tie with filehandle
     - Untie statements
     - Tie with multiple arguments

3. **AST Validation**:
   - [ ] Tie statements produce correct `Tie` node
   - [ ] Variable being tied is captured
   - [ ] Package name is captured
   - [ ] Arguments are captured as children

4. **LSP Integration**:
   - [ ] Tie statements are syntax highlighted correctly
   - [ ] Go-to-definition works for tied package names
   - [ ] Hover shows tie information

### Solution Options

#### Option 1: Full Tie Support (Recommended)

**Pros**:
- Complete coverage of tie statements
- Accurate AST representation
- Enables advanced IDE features

**Cons**:
- More complex implementation
- Tie is a relatively advanced Perl feature

**Implementation**:
```rust
// In NodeKind enum
Tie {
    variable: Variable,
    package: String,
    arguments: Vec<Expression>,
}

// Untie as separate node
Untie {
    variable: Variable,
}
```

#### Option 2: Minimal Tie Recognition

**Pros**:
- Simpler implementation
- Faster to implement
- Reduces NodeKind gap

**Cons**:
- Limited semantic analysis
- Arguments treated as generic expressions

**Implementation**:
```rust
// Simple Tie node with minimal structure
Tie {
    variable: String,  // Variable being tied
    package: String,   // Package name
    args: Vec<Node>,  // Raw argument nodes
}
```

#### Option 3: Tie as Function Call

**Pros**:
- Minimal parser changes
- Reuses existing function call infrastructure

**Cons**:
- Doesn't address the NodeKind gap
- Loses tie-specific semantics
- Untie not recognized

### Path Forward

**Recommended**: Option 1 (Full Tie Support)

**Rationale**:
1. Tie is an important Perl feature for binding variables to objects
2. Complete coverage aligns with the project's "~100% Perl syntax coverage" goal
3. Enables proper IDE support for tie operations
4. The implementation complexity is manageable given existing statement parsing infrastructure

**Implementation Steps**:
1. Add `Tie` and `Untie` variants to `NodeKind` enum
2. Implement tie statement parser with support for:
   - Variable identification (scalar, array, hash, filehandle)
   - Package name parsing
   - Argument list parsing
3. Implement untie statement parser
4. Create comprehensive test fixtures in `test_corpus/`
5. Validate AST structure matches expected format
6. Test with real-world Perl code using tie operations
7. Add LSP integration tests for tie highlighting

**Timeline Estimate**: 1-2 days for implementation + 1 day for testing

### References

- **Perl Documentation**: [perltie - How to hide an object class in a simple variable](https://perldoc.perl.org/perltie)
- **Tie::Array, Tie::Hash, Tie::Scalar**: [Core tie modules](https://perldoc.perl.org/Tie::Array)
- **NodeKind Definition**: `/crates/perl-parser/src/lib.rs` - `NodeKind` enum
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: None currently open
- **GA Feature Alignment**: Tie interface is a P0 critical feature with no coverage

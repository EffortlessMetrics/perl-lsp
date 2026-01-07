# Issue: Format NodeKind Never Seen in Corpus

## Problem Description

### What We Found

The `Format` NodeKind is **never seen** in any corpus test fixture across all four corpus layers:
- Tree-sitter corpus (`tree-sitter-perl/test/corpus/`): 0 occurrences
- Highlight fixtures (`tree-sitter-perl/test/highlight/`): 0 occurrences
- Test corpus (`test_corpus/`): 0 occurrences
- Perl-corpus generators (`crates/perl-corpus/src/gen/`): 0 generators

This represents a **6% gap** in NodeKind coverage (4 of 68 NodeKinds never seen).

### Minimal Reproduction

```perl
# Format statement - a Perl 5 feature for formatted output
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
```

### Current Behavior

When parsing format statements:
- The parser may treat `format` as a bareword or identifier
- No `Format` NodeKind is produced in the AST
- Format picture lines (the lines between `format NAME =` and `.`) are parsed as strings or comments
- Format field specifiers (`@<<<<<<`, `@>>>>>>`, `@###`, etc.) are not recognized as special syntax

This results in:
- Incomplete AST representation of Perl code
- Missing semantic analysis for format statements
- Potential incorrect syntax highlighting
- No IDE support for format-specific features

### Expected Behavior

The parser should:
1. Recognize `format` keyword and parse format statements correctly
2. Produce a `Format` NodeKind in the AST
3. Parse format picture lines as a distinct component
4. Recognize format field specifiers as special syntax
5. Support format variable binding (e.g., `format STDOUT =` vs `format =`)

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/lib.rs` - Add `Format` to `NodeKind` enum
- `/crates/perl-parser/src/parser.rs` - Add format statement parsing logic
- `/crates/perl-parser/src/lexer.rs` - Ensure `format` keyword is recognized

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/format_statements.pl` with comprehensive format examples
- Or create generator in `/crates/perl-corpus/src/gen/format_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] `Format` NodeKind exists in `NodeKind` enum
   - [ ] Parser recognizes `format` keyword
   - [ ] Format picture lines are parsed correctly
   - [ ] Format field specifiers are recognized

2. **Test Coverage**:
   - [ ] At least 5 test cases in corpus covering:
     - Simple format statements
     - Format with picture lines
     - Format with multiple field specifiers
     - Format with variable binding
     - Format with `*_TOP` variants (e.g., `STDOUT_TOP`)

3. **AST Validation**:
   - [ ] Format statements produce correct `Format` node
   - [ ] Picture lines are children of `Format` node
   - [ ] Field specifiers are correctly identified

4. **LSP Integration**:
   - [ ] Format statements are syntax highlighted correctly
   - [ ] Go-to-definition works for format names
   - [ ] Hover shows format information

### Solution Options

#### Option 1: Full Format Statement Support (Recommended)

**Pros**:
- Complete coverage of format statements
- Accurate AST representation
- Enables advanced IDE features

**Cons**:
- More complex implementation
- Format statements are deprecated in Perl (but still widely used)

**Implementation**:
```rust
// In NodeKind enum
Format {
    name: Option<String>,
    picture_lines: Vec<PictureLine>,
}

// In parser
fn parse_format(&mut self) -> Result<Node, ParseError> {
    // Parse format keyword
    // Parse format name (optional)
    // Parse picture lines (until standalone '.')
    // Return Format node
}
```

#### Option 2: Minimal Format Recognition

**Pros**:
- Simpler implementation
- Faster to implement
- Reduces NodeKind gap

**Cons**:
- Limited semantic analysis
- Picture lines treated as generic strings

**Implementation**:
```rust
// Simple Format node with minimal structure
Format {
    name: Option<String>,
    body: String,  // Raw picture lines
}
```

#### Option 3: Format as Special Comment

**Pros**:
- Minimal parser changes
- Treats format as non-executable code

**Cons**:
- Doesn't address the NodeKind gap
- Loses semantic information
- Not a true solution

### Path Forward

**Recommended**: Option 1 (Full Format Statement Support)

**Rationale**:
1. Format statements, while deprecated, are still in active use in legacy Perl codebases
2. Complete coverage aligns with the project's "~100% Perl syntax coverage" goal
3. Enables proper IDE support for developers working with legacy code
4. The implementation complexity is manageable given the existing parser infrastructure

**Implementation Steps**:
1. Add `Format` variant to `NodeKind` enum
2. Implement format statement parser with picture line recognition
3. Create comprehensive test fixtures in `test_corpus/`
4. Validate AST structure matches expected format
5. Test with real-world Perl code using format statements
6. Add LSP integration tests for format highlighting

**Timeline Estimate**: 2-3 days for implementation + 1 day for testing

### References

- **Perl Documentation**: [perlform - Perl formats](https://perldoc.perl.org/perlform)
- **NodeKind Definition**: `/crates/perl-parser/src/lib.rs` - `NodeKind` enum
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: None currently open
- **GA Feature Alignment**: Format statements are a P0 critical feature with no coverage

# Issue: Glob NodeKind Never Seen in Corpus

> **STATUS: ✅ RESOLVED** - Glob NodeKind exists and has corpus coverage.
>
> - `NodeKind::Glob { pattern }` implemented in `crates/perl-parser-core/src/engine/ast.rs`
> - Parser support in `crates/perl-parser-core/src/engine/parser/expressions/primary.rs`
> - Test corpus: `test_corpus/glob_expressions.pl` (added in PR #404)
> - Also covered in: `test_corpus/legacy_syntax.pl`, `test_corpus/advanced_regex.pl`

## Original Problem Description (Historical)

### What We Found (Now Outdated)

~~The `Glob` NodeKind is **never seen** in any corpus test fixture.~~

**Current status**: `NodeKind::Glob { pattern }` exists and is actively used. Test corpus coverage was added in commit 28552903.

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
```

### Current Behavior

When parsing glob expressions:
- The `glob` keyword may be parsed as a bareword function call
- Angle bracket glob syntax `<*.pl>` is parsed as readline operator or generic filehandle
- No `Glob` NodeKind is produced in the AST
- Glob patterns are treated as string literals or unknown syntax

This results in:
- Incomplete AST representation of file pattern matching
- Missing semantic analysis for glob operations
- Potential incorrect syntax highlighting
- No IDE support for glob pattern validation

### Expected Behavior

The parser should:
1. Recognize `glob` as a builtin operator
2. Parse glob patterns correctly (both `glob()` and angle bracket syntax)
3. Produce a `Glob` NodeKind in the AST
4. Support glob pattern syntax (wildcards, character classes, brace expansion)
5. Distinguish glob from similar constructs (readline, filehandle operations)

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/lib.rs` - Add `Glob` to `NodeKind` enum
- `/crates/perl-parser/src/parser.rs` - Add glob parsing logic
- `/crates/perl-parser/src/lexer.rs` - Ensure glob patterns are tokenized correctly

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/glob_expressions.pl` with comprehensive glob examples
- Or create generator in `/crates/perl-corpus/src/gen/glob_gen.rs`

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] `Glob` NodeKind exists in `NodeKind` enum
   - [ ] Parser recognizes `glob()` function syntax
   - [ ] Parser recognizes angle bracket glob syntax `<pattern>`
   - [ ] Glob patterns are parsed correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases in corpus covering:
     - Simple glob patterns (`*.pl`, `*.txt`)
     - Recursive glob patterns (`**/*.pm`)
     - Hidden files (`.*`)
     - Character classes (`[a-z]*`)
     - Brace expansion (`{a,b,c}*`)
     - Nested path patterns
     - Scalar context iteration
     - List context expansion

3. **AST Validation**:
   - [ ] Glob expressions produce correct `Glob` node
   - [ ] Glob pattern is captured as a child node
   - [ ] Context (scalar vs list) is preserved

4. **LSP Integration**:
   - [ ] Glob expressions are syntax highlighted correctly
   - [ ] Hover shows glob pattern information
   - [ ] Completion suggests file patterns

### Solution Options

#### Option 1: Full Glob Support with Pattern Parsing (Recommended)

**Pros**:
- Complete coverage of glob syntax
- Accurate AST representation
- Enables advanced IDE features (pattern validation, file completion)

**Cons**:
- More complex implementation
- Glob patterns have many edge cases

**Implementation**:
```rust
// In NodeKind enum
Glob {
    pattern: Pattern,
    context: GlobContext,  // Scalar or List
}

// Pattern node structure
enum Pattern {
    Wildcard(String),
    Recursive(String),
    CharacterClass(String),
    BraceExpansion(Vec<String>),
    Composite(Vec<Pattern>),
}
```

#### Option 2: Minimal Glob Recognition

**Pros**:
- Simpler implementation
- Faster to implement
- Reduces NodeKind gap

**Cons**:
- Limited semantic analysis
- Glob patterns treated as raw strings

**Implementation**:
```rust
// Simple Glob node with minimal structure
Glob {
    pattern: String,  // Raw pattern string
    angle_bracket: bool,  // Whether using <> syntax
}
```

#### Option 3: Glob as Function Call

**Pros**:
- Minimal parser changes
- Reuses existing function call infrastructure

**Cons**:
- Doesn't address the NodeKind gap
- Loses glob-specific semantics
- Angle bracket syntax still not recognized

### Path Forward

**Recommended**: Option 1 (Full Glob Support with Pattern Parsing)

**Rationale**:
1. Glob is a commonly used Perl feature for file operations
2. Complete coverage aligns with the project's "~100% Perl syntax coverage" goal
3. Enables proper IDE support for file pattern matching
4. The implementation complexity is manageable given existing pattern matching infrastructure

**Implementation Steps**:
1. Add `Glob` variant to `NodeKind` enum
2. Implement glob pattern parser with support for:
   - Wildcards (`*`, `?`)
   - Character classes (`[a-z]`, `[^0-9]`)
   - Brace expansion (`{a,b,c}`)
   - Recursive patterns (`**/`)
3. Handle both `glob()` and angle bracket syntax
4. Create comprehensive test fixtures in `test_corpus/`
5. Validate AST structure matches expected format
6. Test with real-world Perl code using glob operations
7. Add LSP integration tests for glob highlighting

**Timeline Estimate**: 2-3 days for implementation + 1 day for testing

### References

- **Perl Documentation**: [perlfunc - glob](https://perldoc.perl.org/functions/glob)
- **File::Glob**: [Perl core module for glob operations](https://perldoc.perl.org/File::Glob)
- **NodeKind Definition**: `/crates/perl-parser/src/lib.rs` - `NodeKind` enum
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: None currently open
- **GA Feature Alignment**: Glob expressions are a P0 critical feature with no coverage

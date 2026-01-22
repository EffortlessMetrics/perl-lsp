# Issue: Sigil NodeKind Never Seen in Corpus

> **STATUS: ⚠️ NOT A NODEKIND** - This is intentional design, not a coverage gap.
>
> The parser does NOT have a `NodeKind::Sigil` variant. Sigils are captured as a
> `String` field within `NodeKind::Variable { sigil, name }`. This design was chosen
> because sigils are inherently part of variable constructs in Perl, not standalone
> semantic units.
>
> **Design decision**: Option 2 (Keep Sigils as Part of Other Nodes) - see below.

## Original Problem Description (Now Clarified)

### What We Found (Clarification)

~~The `Sigil` NodeKind is **never seen** in any corpus test fixture.~~

**Clarification**: There is no `NodeKind::Sigil` variant in the parser. The parser
has 55 NodeKind variants (not 68 as previously stated), and `Sigil` is not among them.

Sigils (`$`, `@`, `%`, `&`, `*`) are represented as the `sigil: String` field in:
- `NodeKind::Variable { sigil, name }` - for variables like `$foo`, `@array`, `%hash`

This is intentional - sigils are intrinsically part of the variable identifier in Perl.

### Minimal Reproduction

```perl
# Scalar sigils
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');
my &subroutine = \&some_sub;
my *glob = *STDOUT;

# Sigils in dereferencing
my $value = $hash_ref->{key};
my @elements = @$array_ref;
my %data = %$hash_ref;
my $code = &$sub_ref;

# Sigils in typeglobs
local *STDIN = *DATA;
*Package::sub = \&OtherPackage::sub;

# Sigils in special variables
my $0 = $PROGRAM_NAME;
my @ARGV = @ARGV;
my %ENV = %ENV;

# Sigils with package names
my $Package::variable = 'value';
my @Package::array = (1, 2, 3);

# Sigil as standalone (in certain contexts)
$;  # Multidimensional array separator
$!  # Error number
$?  # Child error status
```

### Current Behavior

When parsing sigils:
- Sigils are typically parsed as part of variable names or dereferencing expressions
- No standalone `Sigil` NodeKind is produced in the AST
- The sigil character is not represented as a separate semantic unit
- This may be by design (sigils are part of other constructs)

This results in:
- Incomplete AST representation if sigils should be standalone nodes
- Missing semantic analysis for sigil-specific operations
- Potential inability to distinguish between different uses of the same sigil

### Expected Behavior

**Note**: The `Sigil` NodeKind may be intentionally not used if sigils are better represented as part of other nodes (e.g., `ScalarVariable`, `ArrayVariable`, etc.). If a standalone `Sigil` node is needed:

The parser should:
1. Recognize sigil characters (`$`, `@`, `%`, `&`, `*`) as distinct tokens
2. Optionally produce a `Sigil` NodeKind for standalone sigil usage
3. Distinguish between sigils in different contexts (variable declaration, dereferencing, typeglobs)
4. Support sigil-specific operations and special variables

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/`

**Key Files to Modify**:
- `/crates/perl-parser/src/lib.rs` - Evaluate if `Sigil` should be in `NodeKind` enum
- `/crates/perl-parser/src/parser.rs` - Determine if sigil parsing needs changes
- `/crates/perl-parser/src/lexer.rs` - Ensure sigils are tokenized correctly

**Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/`
- Create `test_corpus/sigil_usage.pl` if standalone sigil nodes are needed
- Or create generator in `/crates/perl-corpus/src/gen/sigil_gen.rs`

### Acceptance Criteria

**Note**: These acceptance criteria assume a standalone `Sigil` node is needed. If sigils are better represented as part of other nodes, this issue should be closed with a note explaining the design decision.

1. **Parser Implementation**:
   - [ ] `Sigil` NodeKind exists in `NodeKind` enum (or documented why not needed)
   - [ ] Parser recognizes sigil characters as distinct tokens
   - [ ] Sigils are parsed correctly in all contexts

2. **Test Coverage** (if Sigil node is needed):
   - [ ] At least 8 test cases in corpus covering:
     - Scalar sigil ($)
     - Array sigil (@)
     - Hash sigil (%)
     - Subroutine sigil (&)
     - Glob sigil (*)
     - Sigils in dereferencing
     - Sigils in typeglobs
     - Special variables with sigils

3. **AST Validation** (if Sigil node is needed):
   - [ ] Sigils produce correct `Sigil` node
   - [ ] Sigil type is captured
   - [ ] Context (variable vs dereference vs typeglob) is preserved

4. **LSP Integration** (if Sigil node is needed):
   - [ ] Sigils are syntax highlighted correctly
   - [ ] Hover shows sigil information
   - [ ] Go-to-definition works for variables with sigils

### Solution Options

#### Option 1: Implement Standalone Sigil Node

**Pros**:
- Complete AST representation
- Enables advanced semantic analysis
- Distinguishes between different sigil uses

**Cons**:
- May add unnecessary complexity
- Sigils are typically part of other constructs
- May duplicate information already in variable nodes

**Implementation**:
```rust
// In NodeKind enum
Sigil {
    kind: SigilKind,  // Scalar, Array, Hash, Subroutine, Glob
    context: SigilContext,  // Variable, Dereference, Typeglob, SpecialVar
}

enum SigilKind {
    Scalar,    // $
    Array,     // @
    Hash,      // %
    Subroutine,// &
    Glob,      // *
}

enum SigilContext {
    VariableDeclaration,
    Dereference,
    Typeglob,
    SpecialVariable,
}
```

#### Option 2: Keep Sigils as Part of Other Nodes (Recommended)

**Pros**:
- Simpler AST structure
- Sigils are inherently part of variable/dereference constructs
- Reduces node count and complexity

**Cons**:
- Doesn't address the NodeKind gap
- May limit some semantic analysis capabilities

**Implementation**:
- No changes needed
- Document that sigils are part of other NodeKinds
- Close this issue with design rationale

#### Option 3: Hybrid Approach

**Pros**:
- Best of both worlds
- Sigils in special contexts get standalone nodes
- Regular variable sigils remain part of variable nodes

**Cons**:
- More complex implementation
- Inconsistent representation

**Implementation**:
```rust
// Sigil node only for special contexts
Sigil {
    kind: SigilKind,
    context: SpecialContext,  // Only for special variables, typeglobs, etc.
}

// Regular variable sigils remain part of variable nodes
ScalarVariable { name: String, ... }  // Includes $ sigil implicitly
```

### Path Forward

**Recommended**: Option 2 (Keep Sigils as Part of Other Nodes)

**Rationale**:
1. Sigils are inherently part of variable/dereference constructs in Perl
2. The current AST structure likely already captures sigil information through other nodes
3. Adding a standalone `Sigil` node would duplicate information and add complexity
4. The NodeKind gap may be intentional design rather than a missing feature

**Implementation Steps**:
1. Review existing AST structure to confirm sigils are captured in other nodes
2. Verify that all sigil contexts are properly represented
3. Document the design decision (why `Sigil` is not a standalone NodeKind)
4. Close this issue with a note explaining the design rationale

**Timeline Estimate**: 1 day for review and documentation

**Alternative**: If analysis reveals that a standalone `Sigil` node is needed, implement Option 1 or Option 3.

### References

- **Perl Documentation**: [perldata - Perl data types](https://perldoc.perl.org/perldata)
- **Perl Sigils**: [Understanding Perl sigils](https://www.perlmonks.org/?node_id=639768)
- **NodeKind Definition**: `/crates/perl-parser/src/lib.rs` - `NodeKind` enum
- **Corpus Structure**: See corpus coverage analysis in `/review/corpus-coverage-*.md`
- **Related Issues**: None currently open
- **GA Feature Alignment**: Sigil usage is a P0 critical feature, but may already be covered by other NodeKinds

# Issue: Indirect Object Syntax Ambiguity

## Problem Description

### What We Found

Perl's indirect object syntax has ambiguity:
- `method $object` - Indirect method call
- `new $object` - Indirect constructor call
- `$object->$method` - Arrow dereference
- Potential for parser confusion

This creates a **P1 high risk** for parser complexity:
- No clear disambiguation rules for indirect object syntax
- Parser may incorrectly parse indirect calls
- No test coverage for indirect object scenarios
- Potential for incorrect AST generation

### Minimal Reproduction

```perl
# Indirect object syntax
my $method = "print";
my $object = "hello";
$method $object;  # Indirect method call

# Indirect constructor
my $class = "MyClass";
my $arg = "argument";
new $class($arg);  # Indirect constructor

# Arrow dereference
my $ref = \$hash;
$ref->{key};  # Arrow dereference
```

### Current Behavior

The parser may:
- Not correctly parse indirect object syntax
- Not disambiguate indirect calls correctly
- Not support all indirect object patterns
- Generate incorrect AST structure
- Return incorrect parse errors or no errors

### Expected Behavior

The parser should:
- Correctly parse indirect object syntax
- Disambiguate indirect calls correctly
- Support all indirect object patterns
- Generate correct AST structure
- Provide clear error messages for ambiguous cases
- Handle edge cases correctly

### Fix Surface

**Parser Module**: `/crates/perl-parser/src/parser.rs` - Main parser logic
- **Lexer Module**: `/crates/perl-lexer/src/lib.rs` - Tokenization with indirect object handling
- **Test Location**: `/test_corpus/` or `/crates/perl-corpus/src/gen/` - Test fixtures for indirect object syntax

### Acceptance Criteria

1. **Parser Implementation**:
   - [ ] Correctly parse indirect object syntax
   - [ ] Disambiguate indirect calls correctly
   - [ ] Support all indirect object patterns
   - [ ] Generate correct AST structure
   - [ ] Provide clear error messages for ambiguous cases
   - [ ] Handle edge cases correctly

2. **Test Coverage**:
   - [ ] At least 8 test cases covering:
     - [ ] Simple indirect method: `$method $object`
     - [ ] Indirect constructor: `new $class($arg)`
     - [ ] Arrow dereference: `$ref->{key}`
     - [ ] Complex indirect object patterns
     - [ ] Edge cases: empty strings, invalid syntax
     - [ ] Error recovery: ambiguous indirect calls
     - [ ] All tests pass

3. **LSP Integration**:
   - [ ] Correct diagnostics for indirect object issues
   - [ ] Hover provides context-aware information
   - [ ] Go-to-definition works correctly for indirect objects
   - [ ] Code completion suggests appropriate indirect object usage

4. **Documentation**:
   - [ ] Indirect object syntax parsing rules documented
   - [ ] Examples of indirect object patterns
   - [ ] API documentation updated with indirect object handling

### Solution Options

#### Option 1: Context-Aware Parsing (Recommended)

**Pros**:
- Comprehensive solution covering all cases
- Clear disambiguation rules
- Recommended approach

**Cons**:
- More complex implementation
- Requires significant parser changes

**Implementation**:
```rust
// Add context tracking to parser
enum IndirectObjectContext {
    MethodCall,
    ConstructorCall,
    ArrowDereference,
    Unknown,
}

// Determine indirect object context based on preceding tokens
fn determine_indirect_object_context(tokens: &[Token]) -> IndirectObjectContext {
    // Look at previous token to determine context
    // If previous token is method keyword, likely method call
    // If previous token is new keyword, likely constructor
    // If previous token is arrow, likely dereference
}
```

#### Option 2: Conservative Fallback

**Pros**:
- Simpler implementation
- Less risk of breaking existing behavior

**Cons**:
- May not handle all edge cases
- Could still have ambiguity in some cases

**Implementation**:
```rust
// Default to method call when ambiguous
fn parse_indirect_object() -> Node {
    // Always parse as method call unless clear context
    parse_method_call()
}
```

#### Option 3: Explicit Disambiguation

**Pros**:
- Clearer intent
- Easier to understand for users

**Cons**:
- Requires changes to Perl syntax
- May not be backward compatible

**Implementation**:
```perl
// Perl could add explicit operators like:
my $result = indirect_method $object;  // Explicit indirect method
my $instance = indirect_new $class($arg);  // Explicit indirect constructor
```

### Path Forward

**Recommended**: Option 1 (Context-Aware Parsing)

**Rationale**:
- Most comprehensive solution
- Handles all edge cases correctly
- Provides best user experience
- Aligns with Perl's actual parsing behavior

**Implementation Steps**:
1. Add `IndirectObjectContext` enum to track context
2. Implement context determination logic in lexer/parser
3. Update indirect object parsing to use context
4. Add test cases for indirect object scenarios
5. Update LSP providers to handle new indirect object nodes
6. Document indirect object syntax parsing rules
7. Validate with existing corpus and real-world code

**Timeline Estimate**: 5-7 days for implementation + 2 days for testing

### References

- **Parser Architecture**: [Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)
- **Lexer Implementation**: `/crates/perl-lexer/src/lib.rs`
- **Parser Implementation**: `/crates/perl-parser/src/parser.rs`
- **LSP Providers**: `/crates/perl-parser/src/features.rs`
- **Corpus Analysis**: Review corpus coverage analysis results
- **Related Issues**: None currently open
- **GA Feature Alignment**: GA feature for indirect object syntax parsing

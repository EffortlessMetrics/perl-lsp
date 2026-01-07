# Issue: Special Contexts Tests for Perl Corpus

**Status**: Open  
**Priority**: P2  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks comprehensive testing of embedded code in special contexts. Perl code can be embedded in various contexts such as regex patterns, format strings, eval expressions, and more. Without testing these special contexts, we cannot ensure the parser correctly handles embedded Perl code.

Without special context tests, we cannot ensure:
- Parser correctly parses embedded code in regex
- Parser correctly parses embedded code in format strings
- Parser correctly parses embedded code in eval expressions
- Parser correctly parses embedded code in other contexts
- LSP provides accurate diagnostics for embedded code
- Symbol resolution works correctly in special contexts

## Impact Assessment

**Why This Matters:**

1. **Embedded Code** - Perl allows code in many contexts
2. **Regex Complexity** - Regex can contain embedded code
3. **Format Strings** - Format strings can contain embedded code
4. **Dynamic Evaluation** - Eval expressions contain embedded code
5. **LSP Accuracy** - Diagnostics must handle embedded code
6. **Real-World Usage** - Many projects use embedded code contexts

**Current State:**
- Limited testing of embedded code contexts
- No comprehensive special context test suite
- Parser may not handle all special contexts
- LSP diagnostics may not work in special contexts
- No tests for symbol resolution in special contexts

## Current State

**What's Missing:**

1. **Embedded regex tests** - Tests for code embedded in regex
2. **Format string tests** - Tests for code in format strings
3. **Eval expression tests** - Tests for code in eval
4. **Special context tests** - Tests for other special contexts
5. **Symbol resolution tests** - Tests for symbol resolution in special contexts
6. **LSP special context tests** - Tests for LSP in special contexts

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) has some regex tests
- [`test_corpus/advanced_regex.pl`](../test_corpus/advanced_regex.pl) has regex tests
- No comprehensive special context test suite
- No special context behavior documentation

## Recommended Path Forward

### Phase 1: Identify Special Contexts

**Objective**: Catalog Perl special contexts

**Steps:**
1. Research Perl special contexts:
   - Regex embedded code: `(?{code})`, `(??{code})`
   - Format strings: `sprintf()`, `printf()`, format strings
   - Eval expressions: `eval "code"`, `eval { code }`
   - Subroutine prototypes: `sub prototype`
   - Attributes: `sub :attr`
   - BEGIN/END blocks: `BEGIN { code }`, `END { code }`
   - CHECK/INIT blocks: `CHECK { code }`, `INIT { code }`
   - UNITCHECK blocks: `UNITCHECK { code }`
2. Document special context behavior:
   - How each context affects parsing
   - What code is allowed in each context
   - How symbols are resolved in each context
   - Context interactions
3. Create special context taxonomy document

**Deliverable**: `docs/perl_special_contexts_taxonomy.md`

### Phase 2: Create Embedded Regex Tests

**Objective**: Test code embedded in regex

**Steps:**
1. Create embedded regex test files:
   ```perl
   # Special context: Embedded code in regex
   # @tags: special_context, regex, embedded_code
   # @perl: 5.10+
   
   use v5.36;
   
   # Test 1: Embedded code in regex
   my $pattern = qr/(?{ $x = 1 })/;
   
   # Test 2: Code assertion in regex
   my $result = "test" =~ /(?{ $x = 1; 1 })/;
   
   # Test 3: Embedded code with variables
   my $var = "test";
   my $result2 = "test" =~ /(?{ $var eq "test" })/;
   
   # Test 4: Nested embedded code
   my $result3 = "test" =~ /(?{ if ($var eq "test") { 1 } else { 0 } })/;
   ```
2. Validate embedded code is correctly parsed
3. Test embedded code edge cases:
   - Multiple embedded code blocks
   - Nested embedded code
   - Embedded code with complex expressions
4. Document embedded regex behavior

**Deliverable**: Embedded regex test suite

### Phase 3: Create Format String Tests

**Objective**: Test code in format strings

**Steps:**
1. Create format string test files:
   ```perl
   # Special context: Format strings
   # @tags: special_context, format_string
   # @perl: 5.00+
   
   use v5.36;
   
   # Test 1: Simple format string
   my $result = sprintf("Hello, %s", "World");
   
   # Test 2: Format string with embedded expressions
   my $name = "World";
   my $result2 = sprintf("Hello, %s", $name);
   
   # Test 3: Format string with complex expressions
   my $x = 10;
   my $y = 20;
   my $result3 = sprintf("Sum: %d", $x + $y);
   
   # Test 4: Format string with hash
   my %hash = (name => "World");
   my $result4 = sprintf("Hello, %s", $hash{name});
   ```
2. Validate format strings are correctly parsed
3. Test format string edge cases:
   - Multiple format specifiers
   - Complex expressions in format strings
   - Nested format strings
4. Document format string behavior

**Deliverable**: Format string test suite

### Phase 4: Create Eval Expression Tests

**Objective**: Test code in eval expressions

**Steps:**
1. Create eval expression test files:
   ```perl
   # Special context: Eval expressions
   # @tags: special_context, eval
   # @perl: 5.00+
   
   use v5.36;
   
   # Test 1: Simple eval
   my $code = 'my $x = 1; $x + 1';
   my $result = eval $code;
   
   # Test 2: Eval with variables
   my $var = 10;
   my $code2 = '$var + 1';
   my $result2 = eval $code2;
   
   # Test 3: Eval with complex expressions
   my $code3 = 'my $x = 1; my $y = 2; $x + $y';
   my $result3 = eval $code3;
   
   # Test 4: Nested eval
   my $code4 = 'eval "1 + 1"';
   my $result4 = eval $code4;
   ```
2. Validate eval expressions are correctly parsed
3. Test eval edge cases:
   - Nested eval expressions
   - Eval with complex code
   - Eval with syntax errors
4. Document eval behavior

**Deliverable**: Eval expression test suite

### Phase 5: Add Symbol Resolution Tests

**Objective**: Test symbol resolution in special contexts

**Steps:**
1. Implement symbol resolution tests:
   ```perl
   # Special context: Symbol resolution
   # @tags: special_context, symbol_resolution
   # @perl: 5.10+
   
   use v5.36;
   
   # Test 1: Symbol resolution in embedded regex
   my $x = 1;
   my $result = "test" =~ /(?{ $x == 1 })/;
   
   # Test 2: Symbol resolution in format strings
   my $name = "World";
   my $result2 = sprintf("Hello, %s", $name);
   
   # Test 3: Symbol resolution in eval
   my $var = 10;
   my $code = '$var + 1';
   my $result3 = eval $code;
   ```
2. Validate symbol resolution is correct
3. Test symbol resolution edge cases:
   - Shadowed symbols
   - Lexical scope in special contexts
   - Package symbols in special contexts
4. Document symbol resolution behavior

**Deliverable**: Symbol resolution test suite

### Phase 6: Add LSP Special Context Tests

**Objective**: Test LSP in special contexts

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with special context tests
2. Implement LSP special context tests:
   - `test_lsp_embedded_regex()` - LSP in embedded regex
   - `test_lsp_format_strings()` - LSP in format strings
   - `test_lsp_eval_expressions()` - LSP in eval
   - `test_lsp_special_context_symbols()` - Symbol resolution in special contexts
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - Correct diagnostics for special contexts
   - Correct symbol resolution in special contexts
   - No false positives for valid code

**Deliverable**: LSP special context test suite

## Priority Level

**P2 - Medium Priority**

This is a P2 issue because:
1. Important but not blocking core functionality
2. Can be addressed incrementally
3. Existing test infrastructure provides foundation
4. Most special contexts are already handled
5. Lower risk than other gaps

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Identify Contexts): 2-3 days
- Phase 2 (Embedded Regex Tests): 3-4 days
- Phase 3 (Format String Tests): 2-3 days
- Phase 4 (Eval Expression Tests): 2-3 days
- Phase 5 (Symbol Resolution Tests): 2-3 days
- Phase 6 (LSP Tests): 2-3 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Error Recovery Tests](corpus-coverage-error-recovery.md) - Related robustness testing

## References

- [perldoc - perlre](https://perldoc.perl.org/perlre) - Regex documentation
- [perldoc - perlfunc](https://perldoc.perl.org/perlfunc) - Function documentation
- [perldoc - perlop](https://perldoc.perl.org/perlop) - Operator documentation

## Success Criteria

1. Special context taxonomy documented with 8-10 contexts
2. Embedded regex tests implemented
3. Format string tests implemented
4. Eval expression tests implemented
5. Symbol resolution tests implemented
6. LSP special context tests implemented
7. Parser correctly handles all special contexts
8. LSP provides accurate diagnostics for special contexts
9. Symbol resolution works correctly in special contexts
10. Special context behavior is documented and tested

## Open Questions

1. Which special contexts are highest priority?
2. How should parser handle nested special contexts?
3. What should happen when special contexts have syntax errors?
4. Should there be version-specific special context tests?
5. What LSP behavior is expected for special contexts?

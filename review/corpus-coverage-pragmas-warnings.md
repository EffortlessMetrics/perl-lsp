# Issue: Pragmas and Warnings Tests for Perl Corpus

**Status**: Open  
**Priority**: P2  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks comprehensive testing of pragma effects on parsing behavior. Perl pragmas (compiler directives) significantly affect how code is parsed and executed, but there is limited test coverage for how the parser handles these directives.

Without pragma and warning tests, we cannot ensure:
- Parser correctly interprets pragma directives
- LSP provides accurate pragma-aware diagnostics
- Pragma changes are correctly reflected in parsing
- Warning categories are properly handled
- Pragma interactions are correctly resolved

## Impact Assessment

**Why This Matters:**

1. **Parsing Behavior** - Pragmas change how Perl parses code
2. **LSP Accuracy** - Diagnostics must be pragma-aware
3. **Code Quality** - Pragmas enforce coding standards
4. **Legacy Support** - Older code uses different pragmas
5. **Modern Perl** - New pragmas enable modern features
6. **User Experience** - Pragma-aware LSP improves developer experience

**Current State:**
- Limited pragma testing in corpus
- No comprehensive pragma behavior documentation
- Parser may not handle all pragma directives
- LSP diagnostics may not be pragma-aware
- No tests for pragma interactions

## Current State

**What's Missing:**

1. **Pragma test files** - Tests for each pragma directive
2. **Warning category tests** - Tests for warning categories
3. **Pragma interaction tests** - Tests for pragma combinations
4. **Pragma scope tests** - Tests for pragma lexical scope
5. **Pragma-aware LSP tests** - Tests for pragma-aware diagnostics
6. **Version-specific pragma tests** - Tests for version-specific pragmas

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) has some pragma tests
- [`test_corpus/modern_perl_features.pl`](../test_corpus/modern_perl_features.pl) has some pragma usage
- No comprehensive pragma test suite
- No pragma behavior documentation

## Recommended Path Forward

### Phase 1: Identify Perl Pragmas

**Objective**: Catalog all Perl pragmas and their effects

**Steps:**
1. Research Perl pragma documentation:
   - `use strict;` - Enforce strict variable declaration
   - `use warnings;` - Enable warnings
   - `use v5.36;` - Version declaration
   - `use feature 'signatures';` - Enable experimental features
   - `no warnings 'deprecated';` - Disable specific warnings
   - `use utf8;` - UTF-8 encoding
   - `use bytes;` - Byte semantics
   - `use integer;` - Integer arithmetic
   - `use lib 'path';` - Library path
   - `use base 'Class';` - Base class
   - `use parent 'Class';` - Parent class
   - `use constant NAME => value;` - Constants
   - `use diagnostics;` - Verbose diagnostics
2. Document pragma effects:
   - How each pragma affects parsing
   - Pragma scope (lexical vs. global)
   - Pragma interactions
   - Version-specific pragmas
3. Create pragma taxonomy document

**Deliverable**: `docs/perl_pragma_taxonomy.md`

### Phase 2: Create Pragma Test Files

**Objective**: Implement tests for each pragma

**Steps:**
1. Create `test_corpus/pragmas/` directory
2. For each pragma, create test file:
   ```perl
   # Pragma: strict
   # @tags: pragma, strict, variables
   # @perl: 5.00+
   
   # Test 1: strict with undeclared variable
   use strict;
   my $declared = 1;
   my $result = $undeclared;  # Error expected
   
   # Test 2: strict with declared variable
   use strict;
   my $declared = 1;
   my $result = $declared;  # OK
   
   # Test 3: strict with package variables
   use strict;
   package MyPackage;
   our $package_var = 1;  # OK
   
   # Test 4: strict with no strict
   use strict;
   {
       no strict 'vars';
       my $undeclared = 1;  # OK
   }
   ```
3. Add pragma-specific metadata tags:
   - `# @tags: pragma, strict, warnings`
4. Validate each pragma is correctly parsed

**Deliverable**: 15-20 pragma test files

### Phase 3: Add Warning Category Tests

**Objective**: Test warning categories

**Steps:**
1. Identify warning categories:
   - `deprecated` - Deprecated features
   - `experimental` - Experimental features
   - `void` - Void context
   - `uninitialized` - Uninitialized variables
   - `once` - Used once
   - `numeric` - Numeric operations
   - `recursion` - Deep recursion
   - `redefine` - Redefining subroutines
2. Create warning category test files:
   ```perl
   # Warning category: deprecated
   # @tags: warning, deprecated
   # @perl: 5.00+
   
   use warnings 'deprecated';
   
   # Deprecated indirect object syntax
   my $fh = new FileHandle;  # Warning expected
   
   # Deprecated bareword filehandle
   open FH, 'file.txt';  # Warning expected
   ```
3. Validate warning categories are correctly detected
4. Test warning category interactions
5. Document warning category behavior

**Deliverable**: Warning category test suite

### Phase 4: Add Pragma Interaction Tests

**Objective**: Test pragma combinations

**Steps:**
1. Identify pragma interactions:
   - `strict` + `warnings`
   - `strict` + `no strict`
   - `warnings` + `no warnings`
   - Multiple `use` statements
   - Pragma scope interactions
2. Create pragma interaction test files:
   ```perl
   # Pragma interaction: strict + warnings
   # @tags: pragma, interaction, strict, warnings
   # @perl: 5.00+
   
   use strict;
   use warnings;
   
   my $declared = 1;
   my $result = $undeclared;  # Error + warning
   ```
3. Validate pragma interactions are correct
4. Test pragma scope interactions
5. Document pragma interaction behavior

**Deliverable**: Pragma interaction test suite

### Phase 5: Add Pragma Scope Tests

**Objective**: Test pragma lexical scope

**Steps:**
1. Implement pragma scope tests:
   ```perl
   # Pragma scope: Lexical scope
   # @tags: pragma, scope, lexical
   # @perl: 5.00+
   
   use strict;
   use warnings;
   
   # Outer scope
   my $outer = 1;
   
   {
       # Inner scope with different pragmas
       no strict 'vars';
       no warnings 'uninitialized';
       
       my $inner = $undeclared;  # No error/warning
   }
   
   # Back to outer scope
   my $result = $undeclared;  # Error expected
   ```
2. Validate pragma scope is correct
3. Test pragma scope edge cases:
   - Nested scopes
   - Overlapping scopes
   - Pragma redeclaration
4. Document pragma scope behavior

**Deliverable**: Pragma scope test suite

### Phase 6: Add Pragma-Aware LSP Tests

**Objective**: Test LSP pragma-aware diagnostics

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with pragma-aware tests
2. Implement pragma-aware diagnostic tests:
   - `test_lsp_pragma_strict()` - Strict variable checking
   - `test_lsp_pragma_warnings()` - Warning detection
   - `test_lsp_pragma_scope()` - Pragma scope handling
   - `test_lsp_pragma_interactions()` - Pragma interactions
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - Correct diagnostics for pragma violations
   - No false positives for valid code
   - Pragma scope is respected
   - Pragma interactions are correct

**Deliverable**: Pragma-aware LSP diagnostic tests

## Priority Level

**P2 - Medium Priority**

This is a P2 issue because:
1. Important but not blocking core functionality
2. Can be addressed incrementally
3. Existing test infrastructure provides foundation
4. Most pragmas are already handled
5. Lower risk than other gaps

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Identify Pragmas): 2-3 days
- Phase 2 (Pragma Tests): 4-5 days
- Phase 3 (Warning Category Tests): 3-4 days
- Phase 4 (Pragma Interaction Tests): 2-3 days
- Phase 5 (Pragma Scope Tests): 2-3 days
- Phase 6 (LSP Tests): 2-3 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Error Recovery Tests](corpus-coverage-error-recovery.md) - Related robustness testing

## References

- [perldoc - strict](https://perldoc.perl.org/strict) - Strict pragma
- [perldoc - warnings](https://perldoc.perl.org/warnings) - Warnings pragma
- [perldoc - perlpragma](https://perldoc.perl.org/perlpragma) - Pragma reference
- [perldoc - perllexwarn](https://perldoc.perl.org/perllexwarn) - Lexical warnings

## Success Criteria

1. Pragma taxonomy documented with 15-20 pragmas
2. 15-20 pragma test files created and validated
3. Warning category tests implemented
4. Pragma interaction tests implemented
5. Pragma scope tests implemented
6. Pragma-aware LSP diagnostic tests implemented
7. Parser correctly handles all pragmas
8. LSP provides accurate pragma-aware diagnostics
9. No pragma-related bugs in existing features
10. Pragma behavior is documented and tested

## Open Questions

1. Which pragmas are highest priority?
2. How should parser handle conflicting pragmas?
3. What should happen when pragmas are declared multiple times?
4. Should there be version-specific pragma tests?
5. What LSP behavior is expected for pragma violations?

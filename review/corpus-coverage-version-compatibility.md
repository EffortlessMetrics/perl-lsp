# Issue: Version Compatibility Tests for Perl Corpus

**Status**: Open  
**Priority**: P2  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks comprehensive version compatibility testing across multiple Perl versions. Perl has evolved significantly from 5.10 through 5.36+, with new features, syntax changes, and deprecated constructs. Without version-specific tests, we cannot ensure the parser correctly handles:

1. **Version-specific syntax** - Features introduced in specific Perl versions
2. **Deprecated syntax** - Constructs removed or deprecated in newer versions
3. **Backward compatibility** - Older code parsing in newer Perl versions
4. **Forward compatibility** - Newer code parsing in older Perl versions
5. **Version detection** - Correctly identifying Perl version from code
6. **Feature detection** - Enabling/disabling features based on version

Without version compatibility tests, we cannot ensure:
- Parser correctly handles all Perl versions
- LSP provides accurate diagnostics for version-specific issues
- Users get correct information about version compatibility
- Parser doesn't break on legacy code
- New features are correctly parsed

## Impact Assessment

**Why This Matters:**

1. **Real-World Usage** - Users run multiple Perl versions
2. **Legacy Code** - Many projects use older Perl versions
3. **Modern Perl** - Users adopt new features (5.36+)
4. **Migration** - Users upgrade Perl versions over time
5. **LSP Accuracy** - Diagnostics must be version-aware
6. **Parser Robustness** - Must handle all Perl versions

**Current State:**
- Limited version-specific testing in corpus
- No comprehensive version compatibility matrix
- Parser may not handle all version-specific features
- LSP diagnostics may not be version-aware
- No tests for version detection accuracy

## Current State

**What's Missing:**

1. **Version-specific test files** - Tests for each Perl version (5.10, 5.12, 5.14, 5.16, 5.18, 5.20, 5.22, 5.24, 5.26, 5.28, 5.30, 5.32, 5.34, 5.36)
2. **Feature introduction tests** - Tests for features introduced in each version
3. **Deprecated syntax tests** - Tests for deprecated/removed constructs
4. **Version detection tests** - Tests for version detection accuracy
5. **Backward compatibility tests** - Tests for parsing older code in newer versions
6. **Forward compatibility tests** - Tests for parsing newer code in older versions
7. **Version-aware diagnostics** - Tests for LSP diagnostics based on version

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) has some version-specific tests
- [`test_corpus/packages_versions.pl`](../test_corpus/packages_versions.pl) has package/version tests
- [`test_corpus/modern_perl_features.pl`](../test_corpus/modern_perl_features.pl) has modern Perl tests
- No comprehensive version compatibility matrix

## Recommended Path Forward

### Phase 1: Create Version Compatibility Matrix

**Objective**: Document Perl version features and changes

**Steps:**
1. Research Perl version history:
   - Perl 5.10 (2007): say, given/when, smart match
   - Perl 5.12 (2010): package block syntax, Yada Yada
   - Perl 5.14 (2011): Non-destructive substitution, /r modifier
   - Perl 5.16 (2012): __SUB__, current_sub, lexical_subs
   - Perl 5.18 (2013): Computed labels, lexical_subs
   - Perl 5.20 (2014): Subroutine signatures (experimental), slice aliasing
   - Perl 5.22 (2015): Bitwise operators, //=
   - Perl 5.24 (2016): postfix dereferencing, /xx modifier
   - Perl 5.26 (2017): Indented heredocs, lexical_subs
   - Perl 5.28 (2018): Unicode 10.0, key/value slice
   - Perl 5.30 (2019): Unicode 12.1, no indirect object syntax
   - Perl 5.32 (2020): Unicode 13.0, isa operator
   - Perl 5.34 (2021): try/catch (experimental), Unicode 14.0
   - Perl 5.36 (2022): signatures, try/catch, defer
2. Create version compatibility matrix:
   - Features introduced in each version
   - Syntax deprecated in each version
   - Syntax removed in each version
   - Breaking changes between versions
3. Document version detection methods:
   - `use v5.36;` - Version declaration
   - `use 5.036;` - Alternative version declaration
   - `require v5.36;` - Runtime version check
4. Create version taxonomy document

**Deliverable**: `docs/perl_version_compatibility_matrix.md`

### Phase 2: Create Version-Specific Test Files

**Objective**: Implement tests for each Perl version

**Steps:**
1. Create `test_corpus/version_compatibility/` directory
2. For each Perl version, create test file:
   ```perl
   # Version compatibility: Perl 5.36 features
   # @tags: version, 5.36, features
   # @perl: 5.36+
   
   use v5.36;
   use strict;
   use warnings;
   
   # Feature: Signatures (stable in 5.36)
   sub greet($name) {
       return "Hello, $name";
   }
   
   # Feature: try/catch/defer (stable in 5.36)
   try {
       greet("World");
   } catch ($e) {
       warn "Error: $e";
   }
   
   # Feature: isa operator (introduced in 5.32, stable in 5.36)
   my $obj = bless {}, "MyClass";
   if ($obj isa MyClass) {
       say "Object is MyClass";
   }
   
   # Feature: postfix dereferencing (introduced in 5.24)
   my @array = (1, 2, 3);
   my $element = $array[0];  # Traditional
   my $element2 = $array->[0];  # Postfix
   
   # Feature: key/value slice (introduced in 5.28)
   my %hash = (a => 1, b => 2);
   my @keys = %hash{'a', 'b'};  # Key slice
   my @values = %hash{<a b>};  # Value slice
   ```
3. Add version-specific metadata tags:
   - `# @perl: 5.36+` - Minimum version required
   - `# @tags: version, 5.36, features`
4. Validate each feature is correctly parsed

**Deliverable**: 14 version-specific test files (5.10-5.36)

### Phase 3: Add Deprecated Syntax Tests

**Objective**: Test deprecated and removed syntax

**Steps:**
1. Identify deprecated/removed syntax:
   - Indirect object syntax (deprecated in 5.32, removed in 5.36)
   - `::=` syntax (deprecated)
   - `$/` as input record separator (discouraged)
   - `$"` as list separator (discouraged)
2. Create deprecated syntax test files:
   ```perl
   # Deprecated syntax: Indirect object syntax
   # @tags: deprecated, syntax, indirect_object
   # @perl: 5.10-5.34
   # @deprecated: 5.32
   # @removed: 5.36
   
   use v5.30;
   
   # Indirect object syntax (deprecated in 5.32)
   my $fh = new FileHandle;  # Deprecated
   # Should be: my $fh = FileHandle->new;
   
   # Indirect object syntax with method
   my $result = $obj->method();  # OK
   my $result2 = method $obj;  # Deprecated
   ```
3. Validate parser handles deprecated syntax correctly
4. Add LSP diagnostic tests for deprecated syntax
5. Document deprecated syntax behavior

**Deliverable**: Deprecated syntax test suite

### Phase 4: Add Version Detection Tests

**Objective**: Test version detection accuracy

**Steps:**
1. Implement version detection tests:
   ```perl
   # Version detection: Various version declarations
   # @tags: version, detection
   # @perl: 5.10+
   
   # Test 1: Version declaration with v-string
   use v5.36;
   
   # Test 2: Version declaration with numeric
   use 5.036;
   
   # Test 3: Version declaration with require
   require v5.36;
   
   # Test 4: Version check in code
   if ($] >= 5.036) {
       say "Perl 5.36 or later";
   }
   
   # Test 5: Version check with $^V
   if ($^V ge v5.36.0) {
       say "Perl 5.36 or later";
   }
   ```
2. Validate version detection is accurate
3. Test version detection edge cases:
   - Multiple version declarations
   - Conflicting version declarations
   - No version declaration
4. Document version detection behavior

**Deliverable**: Version detection test suite

### Phase 5: Add Backward Compatibility Tests

**Objective**: Test parsing older code in newer versions

**Steps:**
1. Create backward compatibility test scenarios:
   - Perl 5.10 code parsed in 5.36
   - Legacy syntax in modern Perl
   - Deprecated constructs still working
2. Implement backward compatibility tests:
   ```perl
   # Backward compatibility: Legacy code in modern Perl
   # @tags: backward_compat, legacy
   # @perl: 5.36+
   
   use v5.36;
   
   # Legacy syntax that still works
   my @array;
   push @array, 1, 2, 3;  # Legacy push syntax
   
   # Legacy variable declarations
   my $var;  # OK
   my ($a, $b) = (1, 2);  # OK
   
   # Legacy regex syntax
   if ($string =~ /pattern/) {  # OK
       say "Match";
   }
   ```
3. Validate backward compatibility is maintained
4. Test LSP provides accurate diagnostics for legacy code
5. Document backward compatibility behavior

**Deliverable**: Backward compatibility test suite

### Phase 6: Add Version-Aware LSP Diagnostics

**Objective**: Test LSP diagnostics based on version

**Steps:**
1. Extend [`crates/perl-lsp/tests/`](../crates/perl-lsp/tests/) with version-aware tests
2. Implement version-aware diagnostic tests:
   - `test_lsp_version_deprecated_syntax()` - Diagnostics for deprecated syntax
   - `test_lsp_version_feature_unavailable()` - Diagnostics for unavailable features
   - `test_lsp_version_compatibility()` - Version compatibility warnings
3. Use LSP test harness for realistic editor interactions
4. Validate:
   - Correct diagnostics for version-specific issues
   - No false positives for valid code
   - Helpful suggestions for version compatibility

**Deliverable**: Version-aware LSP diagnostic tests

## Priority Level

**P2 - Medium Priority**

This is a P2 issue because:
1. Important but not blocking core functionality
2. Can be addressed incrementally
3. Existing test infrastructure provides foundation
4. Most version-specific features are already handled
5. Lower risk than other gaps

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Version Matrix): 2-3 days
- Phase 2 (Version-Specific Tests): 5-7 days
- Phase 3 (Deprecated Syntax Tests): 3-4 days
- Phase 4 (Version Detection Tests): 2-3 days
- Phase 5 (Backward Compatibility Tests): 3-4 days
- Phase 6 (LSP Diagnostics): 2-3 days

## Related Issues

- [Integration Tests](corpus-coverage-integration-tests.md) - Related corpus testing
- [Error Recovery Tests](corpus-coverage-error-recovery.md) - Related robustness testing

## References

- [Perl 5.36 Release Notes](https://metacpan.org/pod/perl5360delta) - Latest Perl changes
- [Perl 5.10 Release Notes](https://metacpan.org/pod/perl5100delta) - Early Perl features
- [perldoc - perlvar](https://perldoc.perl.org/perlvar) - Perl variables
- [perldoc - perlsyn](https://perldoc.perl.org/perlsyn) - Perl syntax

## Success Criteria

1. Version compatibility matrix documented for all Perl versions (5.10-5.36)
2. 14 version-specific test files created and validated
3. Deprecated syntax tests implemented
4. Version detection tests implemented
5. Backward compatibility tests implemented
6. Version-aware LSP diagnostic tests implemented
7. Parser correctly handles all Perl versions
8. LSP provides accurate version-specific diagnostics
9. No version-related bugs in existing features
10. Version compatibility is documented and tested

## Open Questions

1. Which Perl versions are highest priority?
2. Should version compatibility tests be version-specific or feature-specific?
3. How should parser handle code without version declaration?
4. Should there be version-specific LSP behavior?
5. What should happen when code uses features from multiple versions?

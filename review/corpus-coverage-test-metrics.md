# Issue: Test Coverage Metrics for Perl Corpus

**Status**: Open  
**Priority**: P1  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks clear, quantitative metrics on test coverage percentage and specific gaps in syntax coverage. While the corpus contains extensive test files covering many Perl 5 syntax features, there is no systematic way to measure:

1. **Overall coverage percentage** - What percentage of Perl 5 syntax is actually covered
2. **Feature-level coverage** - Which specific Perl features have adequate test coverage
3. **Gap identification** - Which syntax constructs are missing or under-tested
4. **Coverage trends** - How coverage changes over time as new features are added

Without these metrics, it is difficult to:
- Prioritize which syntax areas need additional test coverage
- Measure progress toward comprehensive Perl 5 syntax support
- Identify regression risks when making parser changes
- Validate claims of "~100% Perl 5 syntax coverage"

## Impact Assessment

**Why This Matters:**

1. **Quality Assurance**: Without coverage metrics, we cannot scientifically validate the parser's claim of "~100% Perl 5 syntax coverage"
2. **Risk Mitigation**: Changes to the parser could inadvertently break previously working syntax without detection
3. **Resource Allocation**: Cannot make informed decisions about where to focus testing efforts
4. **Release Confidence**: Cannot provide quantitative evidence of parser robustness to users
5. **Benchmarking**: Cannot compare coverage against other Perl parsers (e.g., PPI, Perl::Tidy)

**Current State:**
- The corpus contains test files for: advanced regex, source filters, XS/FFI, modern Perl features, legacy syntax, data/end sections, packages/versions
- No automated coverage reporting exists
- Coverage claims are qualitative, not quantitative
- No baseline metrics established for regression detection

## Current State

**What's Missing:**

1. **Coverage Reporting Tool**: No automated tool to calculate syntax coverage percentage
2. **Feature Taxonomy**: No comprehensive taxonomy of Perl 5 syntax features to measure against
3. **Gap Detection**: No systematic method to identify untested or under-tested syntax constructs
4. **Baseline Metrics**: No established baseline coverage metrics for comparison
5. **Trend Tracking**: No mechanism to track coverage changes over time
6. **Regression Detection**: No automated detection of coverage regressions after parser changes

**Existing Infrastructure:**
- [`test_corpus/`](../test_corpus/) directory contains 9 test files
- [`crates/perl-corpus/`](../crates/perl-corpus/) provides corpus parsing infrastructure
- Corpus files use section-based format with metadata tags
- No coverage calculation or reporting utilities exist

## Recommended Path Forward

### Phase 1: Establish Feature Taxonomy

**Objective**: Create a comprehensive taxonomy of Perl 5 syntax features

**Steps:**
1. Research Perl 5.36+ syntax specification and perldoc
2. Create taxonomy document covering:
   - Core language features (variables, operators, control flow, subroutines)
   - Advanced features (regex, quote-like operators, heredocs, format strings)
   - Modern features (signatures, try/catch/defer, class/field, builtin::)
   - Legacy features (bareword filehandles, indirect object syntax)
   - XS and FFI patterns
   - Source filters and code generation
   - Pragmas and version-specific features
3. Map each feature to test corpus entries
4. Identify features without corresponding tests

**Deliverable**: `docs/perl5_syntax_taxonomy.md`

### Phase 2: Implement Coverage Calculation

**Objective**: Create automated tool to calculate syntax coverage

**Steps:**
1. Extend [`crates/perl-corpus/src/lib.rs`](../crates/perl-corpus/src/lib.rs) with coverage module
2. Implement `calculate_coverage()` function that:
   - Parses all corpus test files
   - Extracts syntax features using metadata tags
   - Maps features to taxonomy
   - Calculates coverage percentage: `(tested_features / total_features) * 100`
3. Generate coverage report with:
   - Overall coverage percentage
   - Coverage by feature category
   - List of untested features
   - List of under-tested features (< 3 test cases)
4. Add CLI command: `cargo run --bin perl-corpus -- coverage`

**Deliverable**: Coverage calculation module in [`crates/perl-corpus/`](../crates/perl-corpus/)

### Phase 3: Establish Baseline Metrics

**Objective**: Capture initial coverage baseline

**Steps:**
1. Run coverage calculation on current corpus
2. Document baseline metrics in `docs/coverage_baseline.md`
3. Identify critical gaps (P0 features with < 50% coverage)
4. Prioritize gap remediation based on feature importance
5. Create GitHub issue tracking for each critical gap

**Deliverable**: Baseline coverage report

### Phase 4: Integrate with CI

**Objective**: Automate coverage tracking in CI pipeline

**Steps:**
1. Add coverage calculation to CI workflow
2. Fail CI if coverage drops below threshold (e.g., 95%)
3. Generate coverage badge for README
4. Add coverage trend visualization
5. Notify on coverage regressions

**Deliverable**: CI integration for coverage tracking

### Phase 5: Gap Remediation

**Objective**: Address identified coverage gaps

**Steps:**
1. Create test files for missing features
2. Enhance existing tests for under-tested features
3. Add regression tests for edge cases
4. Validate coverage improvements
5. Update baseline metrics

**Deliverable**: Enhanced test corpus with improved coverage

## Priority Level

**P1 - High Priority**

This is a P1 issue because:
1. Foundation for all other corpus quality improvements
2. Enables data-driven decisions about testing priorities
3. Critical for validating parser robustness claims
4. Required for production readiness assessment
5. Blocks meaningful progress measurement

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Feature Taxonomy): 2-3 days
- Phase 2 (Coverage Calculation): 3-4 days
- Phase 3 (Baseline Metrics): 1-2 days
- Phase 4 (CI Integration): 2-3 days
- Phase 5 (Gap Remediation): Ongoing, 5-10 days for initial gaps

## Related Issues

- None directly related, but this issue enables better tracking of all corpus-related work
- Supports validation of existing syntax coverage claims

## References

- [`test_corpus/README.md`](../test_corpus/README.md) - Current corpus documentation
- [`crates/perl-corpus/src/lib.rs`](../crates/perl-corpus/src/lib.rs) - Corpus parsing infrastructure
- [perldoc - perlfunc](https://perldoc.perl.org/perlfunc) - Perl function reference
- [Perl 5.36+ Release Notes](https://metacpan.org/pod/perl5260delta) - Modern Perl features

## Success Criteria

1. Feature taxonomy document created and reviewed
2. Coverage calculation tool implemented and tested
3. Baseline metrics established and documented
4. CI integration automated and passing
5. Initial coverage gaps identified and prioritized
6. Coverage badge displayed in README
7. Coverage regressions detected and reported

## Open Questions

1. What coverage threshold should be considered "production ready"? (Suggested: 95%)
2. Should coverage be weighted by feature importance? (e.g., modern features > legacy)
3. How should coverage be calculated for version-specific features?
4. Should there be different coverage targets for different Perl versions?

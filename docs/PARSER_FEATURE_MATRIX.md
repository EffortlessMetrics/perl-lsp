# Parser Feature Matrix

> **Issue #180**: This document tracks parser coverage and missing features.
> Auto-generated from `corpus_audit_report.json` on 2026-01-09 04:23

## Summary

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Parse Success Rate | 40% (4/10 files) | 100% | In Progress |
| Parse Errors | 6 | 0 | Baseline Set |
| Timeouts | 0 | 0 | Passing |
| Panics | 0 | 0 | Passing |
| GA Feature Coverage | 100% | 100% | Passing |
| Baseline | 6 | 0 | Ratcheted |

## Error Breakdown by Category

Errors are categorized to help prioritize improvements:

| Category | Count | Priority | Description |
|----------|-------|----------|-------------|
| ModernFeature | 5 | P1 | class/try/catch/field/method keywords |
| Subroutine | 1 | P2 | Signatures, prototypes |

## Failing Files

| File | Category | Error Summary |
|------|----------|---------------|
| ./test_corpus/legacy_syntax.pl | ModernFeature | Unexpected token: expected expression, found Co... |
| ./test_corpus/modern_perl_features.pl | ModernFeature | Unexpected token: expected RightParen, found Id... |
| ./test_corpus/packages_versions.pl | ModernFeature | Unexpected token: expected RightBrace, found Id... |
| ./test_corpus/real_world/medium_module.pl | ModernFeature | Unexpected token: expected expression, found Or... |
| ./test_corpus/source_filters.pl | Subroutine | Unexpected token: expected RightBrace, found Se... |
| ./test_corpus/xs_inline_ffi.pl | ModernFeature | Unexpected token: expected expression, found Fa... |

## Coverage Roadmap

### Phase 1: Stabilize Core (Current)
- [x] Establish baseline ratchet (Issue #180)
- [x] Add error categorization
- [ ] Reduce parse errors to 0

### Phase 2: Modern Perl Features
- [ ] `class` keyword (Perl 5.38+, Corinna)
- [ ] `try`/`catch`/`finally` blocks
- [ ] `field` and `method` declarations
- [ ] `builtin::` functions

### Phase 3: Edge Cases
- [ ] Complex heredoc scenarios
- [ ] Unicode in quote delimiters
- [ ] Recursive regex patterns

## How to Use

```bash
# View current parse status
just parser-audit

# Check against baseline (CI mode)
just ci-parser-features-check

# Update this document from latest audit
just parser-matrix-update
```

## Baseline Ratchet

The parse error count uses a ratchet mechanism:
- Baseline stored in `ci/parse_errors_baseline.txt`
- CI fails if parse errors **increase**
- CI passes if parse errors stay same or decrease
- When errors decrease, update baseline: `echo N > ci/parse_errors_baseline.txt`

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Project overview and commands
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture
- [features.toml](../features.toml) - LSP feature catalog

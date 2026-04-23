# Parser Feature Matrix

> **Issue #180**: This document tracks parser coverage and missing features.

## Provenance

| Field | Value |
|-------|-------|
| Generated | 2026-03-26 23:31 |
| Commit | `d7f7198e` |
| perl-parser | vunknown |
| Corpus | `test_corpus/` |
| Command | `just parser-audit && just parser-matrix-update` |

## Summary

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Parse Success Rate | 100% (91/91) | 100% | Passing |
| Parse Errors | 0 | 0 | Passing |
| Timeouts | 0 | 0 | Passing |
| Panics | 0 | 0 | Passing |
| Test Corpus Inventory | 100% | 100% | Passing |
| Baseline | 0 | 0 | Ratcheted |

*Test Corpus Inventory* measures whether the test corpus contains examples of each
GA (generally available) feature defined in `features.toml`. It does NOT measure
whether those features parse successfully—that's what Parse Success Rate tracks.

## Error Breakdown by Category

Errors are categorized to help prioritize implementation work:

| Category | Count | Priority | Description |
|----------|-------|----------|-------------|
| ControlFlow | 0 | P2 | given/when/default |
| Dereference | 0 | P2 | ->, postfix deref |
| General | 0 | P3 | Uncategorized |
| ModernFeature | 0 | P1 | class/try/catch/field/method keywords |
| QuoteLike | 0 | P2 | q/qq/qw/qx/qr, heredocs, strings |
| Regex | 0 | P2 | m//, s///, tr///, patterns |
| Subroutine | 0 | P2 | Signatures, prototypes |

## Failing Files

*No failing files* ✅

## Coverage Roadmap

### Phase 1: Stabilize Core (Current)
- [x] Establish baseline ratchet (Issue #180)
- [x] Add error categorization
- [x] Reduce parse errors to 0

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

**Philosophy**: Baseline updates are only allowed when the parser actually improves
(error count decreases), never to paper over regressions. The ratchet ensures the
codebase only gets easier to reason about over time.

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Project overview and commands
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture
- [features.toml](../features.toml) - LSP feature catalog

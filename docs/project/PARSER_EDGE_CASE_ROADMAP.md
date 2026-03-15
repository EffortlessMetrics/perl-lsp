# Parser Edge Case Roadmap

> **Baseline**: 7,095 `.pm` files from system Perl (5.038002) + CPAN modules.
> **Date**: 2026-03-09
> **Method**: First-error-per-file analysis to identify root causes (not cascades).
> **Tool**: `just corpus-sweep` (automated via `xtask parser-corpus-sweep`)

---

## Current State (master, post-Wave 1)

| Metric | Value |
|--------|-------|
| Total .pm files scanned | 7,095 |
| Unreadable (encoding) | 48 |
| Clean files (0 errors) | 3,627 (51.1%) |
| Files with errors | 3,420 (48.2%) |
| Unique first-error buckets | 26 |
| Total ERROR nodes | 66,771 |

Most errors cascade: a single misparse triggers 10-20 downstream `ERROR` nodes.

---

## Wave 1 — Merged (PRs #1215-#1218)

| PR | Fix | Files Fixed | Status |
|----|-----|-------------|--------|
| #1215 | POD block skipping in lexer | ~333 | Merged |
| #1216 | Regex false positive nested quantifiers | ~22 | Merged |
| #1217 | `&{expr}` code dereference | overlap | Merged |
| #1218 | Expand builtins + forward declarations | ~120 | Merged |

**Result**: 3,627 / 7,095 clean (51.1%) — baseline established

---

## Wave 2 — High-Impact Single Fixes (~500 files)

> **Updated**: 2026-03-14 (agent swarm session results)

### 2A. Package-Qualified Array/Hash Subscript (261 files)

**Status**: ALREADY WORKING (PR #1481 — 9 regression tests added)

**Error**: `expected RightBracket, found Identifier`

**Construct**: `$Package::Name[index]` — package-qualified array element access.

**Example**: `$Text::Unidecode::Char[0xff] = [...]` (256 files from Text::Unidecode alone)

**Investigation result**: The parser's `parse_postfix()` loop already handles `[...]` and `{...}` after qualified variables correctly. The 261 file count may stem from a different root cause (e.g., cascading errors from other parse failures). Re-analyze after Wave 2B/2C merges to determine the true remaining impact.

### 2B. Fat Arrow (`=>`) as General Separator (91 files)

**Status**: FIXED (PR #1484 — 3 code paths + 6 tests)

**Error**: `expected expression, found FatArrow`

**Construct**: `=>` used where `,` would go — valid Perl, auto-quotes LHS.

**Examples**:
```perl
push @array => $value;          # push with =>
bless \%opts => $class;         # bless with =>
push @attrs => (key => $val);   # nested fat arrows
```

**Root cause**: Three code paths in `statements.rs` where `=>` was not accepted as a list separator: (1) `tie` argument lists, (2) `map`/`grep`/`sort` block-then-list parsing, (3) remaining-args loop after initial arguments. All three fixed.

### 2C. `split /regex/` — Slash After Builtin (22 files)

**Status**: ALREADY FIXED (commit `88e325ee`, PR #1468 — 19 regression tests added)

**Error**: `expected expression, found Slash`

**Construct**: `split /pattern/, $string` — regex literal after `split`.

**Examples**:
```perl
split /\./, $Config{osvers};
split /\s+/, $cmd;
split /;/, $ENV{LIB};
```

**Investigation result**: Commit `88e325ee` already handles `relex_as_term()` in both statement and expression contexts. The slash-after-builtin disambiguation was resolved prior to this session.

### 2D. Statement Modifiers After Complex Expressions (41 files)

**Status**: ALREADY WORKING (PR #1485 — 35 regression tests added)

**Error**: `expected RightBrace, found Identifier`

**Construct**: Postfix `if`/`unless`/`while`/`for` after complex statements.

**Examples**:
```perl
push @{$found{$type}}, $item;  # then } if/unless/while
$cflags{$_} ||= '';            # then if/for modifier
```

**Investigation result**: The parser's `is_at_statement_end()` correctly includes modifier keywords as statement boundaries. The 41 file count was likely from cascading errors rather than a genuine modifier-parsing gap.

### Wave 2 Summary

| Item | Original Estimate | Outcome | PR |
|------|------------------|---------|----|
| 2A | 261 files | Already working; needs re-count | #1481 |
| 2B | 91 files | Fixed (3 code paths) | #1484 |
| 2C | 22 files | Already fixed | #1468 |
| 2D | 41 files | Already working; needs re-count | #1485 |

**After Wave 2**: Re-run `just corpus-sweep` after merging all PRs to measure actual improvement. Many of the original file counts may have been from cascading errors now resolved by other fixes.

---

## Wave 3 — Expression Parsing Gaps (~300 files)

### 3A. Parenthesized Assignment with Regex Bind (~50 files)

**Error**: `expected RightParen, found Identifier`

**Construct**: `(my $var = $expr) =~ s/foo/bar/`

**Root cause**: The parser doesn't handle assignment inside parentheses creating an lvalue for `=~`.

### 3B. `for`/`foreach` with Block-Taking Builtins (~50 files)

**Error**: `expected RightParen, found Identifier`

**Construct**: `for my $x (map { ... } @list) { ... }`

**Root cause**: `map`/`grep` blocks inside the iterator expression of a `for` loop confuse brace matching.

### 3C. Complex Ternary `? :` Expressions (~9 files)

**Error**: `expected expression, found Question`

**Construct**: Multi-line ternary with complex operands.

**Examples**:
```perl
exists $me->{login}
    ? $me->{login}
    : undef;
```

### 3D. `use overload` with Operator Strings (~20 files)

**Construct**: `use overload '""' => \&stringify, '0+' => \&numify, fallback => 1;`

### 3E. Chained `->method()` After Certain Constructs (~41 files)

**Status**: FIXED (PR #1474)

**Error**: `expected expression, found Arrow`

**Root cause**: Arrow deref subscripts (`->[]`, `->{}`) did not properly continue the postfix chain, so subsequent `->method()` calls after a deref were rejected. Fixed to allow the postfix loop to continue after arrow-deref subscripts.

### 3F. Complex List/Hash Construction in Args (~45 files)

**Error**: `expected expression, found Comma`

**Construct**: Multi-expression list arguments with mixed commas.

**After Wave 3**: measured after landing

---

## Wave 4 — Long Tail (~150 files)

> **Updated**: 2026-03-14 (agent swarm session results)

| Category | Files | Example | Status |
|----------|-------|---------|--------|
| 4A. Control flow in expressions | ~19 | `return`/`next`/`last`/`redo` in ternary/short-circuit | FIXED (PR #1483) |
| `eval` block edge cases | ~5 | Nested eval with complex error handling | Open |
| `goto` in expression context | ~3 | `goto &subroutine` | Open |
| 4C. Unclosed block recovery | ~30 | `RightBrace at Eof` cascade | IMPROVED (PR #1487) |
| Miscellaneous (each <5 files) | ~90 | Various rare constructs | Open |

### 4A. Control Flow in Expressions (~19 files)

**Status**: FIXED (PR #1483)

`return`, `next`, `last`, and `redo` now parse correctly as expressions inside ternary branches (`? return $x : $y`) and short-circuit operators (`$ok || return`). Previously these were only recognized as statement-level constructs.

### 4C. Unclosed Block Recovery (~30 files)

**Status**: IMPROVED (PR #1487)

Parser now returns a partial AST when encountering a missing `}` at EOF instead of failing entirely. This reduces cascading errors from unclosed blocks and improves diagnostics for incomplete code.

**After Wave 4**: measured after landing

---

## Validation Method

```bash
# Run corpus sweep (automated harness)
just corpus-sweep

# Check against committed baseline (fails on regression)
just corpus-sweep-check

# Update baseline after improvements
just corpus-sweep-update

# Verbose mode (per-file details)
cargo run -p xtask -- parser-corpus-sweep --verbose
```

---

## Priority Ordering

| Wave | Effort | Impact | Clean Rate | Status |
|------|--------|--------|------------|--------|
| 1 (done) | 4 merged PRs (#1215-#1218) | baseline | 51.1% | Merged |
| 2 | 1 fix + 3 already working | re-count needed | pending sweep | PRs #1468, #1481, #1484, #1485 |
| 3 | 6 expression fixes (1 fixed) | +300 files | pending sweep | PR #1474 (3E) |
| 4 | Long tail (2 fixed/improved) | +150 files | pending sweep | PRs #1483 (4A), #1487 (4C) |

### Session Summary (2026-03-14)

The agent swarm session investigated all 4 Wave 2 items plus additional edge cases. Key findings:

- **3 of 4 Wave 2 items were already working** — the original file counts were likely inflated by cascading errors from other parse failures
- **1 Wave 2 item (2B) had a genuine bug** fixed across 3 code paths
- **3 additional fixes** landed from Waves 3 and 4 (chained deref, control flow expressions, unclosed block recovery)
- **Total PRs from session**: 7 (#1468, #1474, #1481, #1483, #1484, #1485, #1487)
- **Next step**: After merging, run `just corpus-sweep` to get updated clean file counts and reassess remaining work

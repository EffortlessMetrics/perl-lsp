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

### 2A. Package-Qualified Array/Hash Subscript (261 files)

**Error**: `expected RightBracket, found Identifier`

**Construct**: `$Package::Name[index]` — package-qualified array element access.

**Example**: `$Text::Unidecode::Char[0xff] = [...]` (256 files from Text::Unidecode alone)

**Root cause**: The parser resolves `$Text::Unidecode::Char` as a qualified variable name but then doesn't recognize `[0xff]` as an array subscript because it expects the variable to end after the qualified name.

**Fix location**: Variable parsing — after resolving a package-qualified variable, check for `[` (array subscript) or `{` (hash subscript) and parse accordingly.

**Impact**: Single fix, 261 files — the largest single-fix win remaining.

### 2B. Fat Arrow (`=>`) as General Separator (91 files)

**Error**: `expected expression, found FatArrow`

**Construct**: `=>` used where `,` would go — valid Perl, auto-quotes LHS.

**Examples**:
```perl
push @array => $value;          # push with =>
bless \%opts => $class;         # bless with =>
push @attrs => (key => $val);   # nested fat arrows
```

**Root cause**: Certain builtins/contexts expect `,`-separated lists but don't accept `=>` as an equivalent separator.

**Fix location**: Expression list parsing — treat `=>` as list separator in function argument contexts.

### 2C. `split /regex/` — Slash After Builtin (22 files)

**Error**: `expected expression, found Slash`

**Construct**: `split /pattern/, $string` — regex literal after `split`.

**Examples**:
```perl
split /\./, $Config{osvers};
split /\s+/, $cmd;
split /;/, $ENV{LIB};
```

**Root cause**: After `split`, `/` is treated as division operator rather than regex delimiter.

**Fix location**: `parse_simple_statement()` — when parsing `split`, expect a regex as first argument.

### 2D. Statement Modifiers After Complex Expressions (41 files)

**Error**: `expected RightBrace, found Identifier`

**Construct**: Postfix `if`/`unless`/`while`/`for` after complex statements.

**Examples**:
```perl
push @{$found{$type}}, $item;  # then } if/unless/while
$cflags{$_} ||= '';            # then if/for modifier
```

**Root cause**: After parsing a complex expression with braces, the parser consumes the closing `}` but doesn't check for trailing statement modifiers.

**After Wave 2**: measured after landing

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

**Error**: `expected expression, found Arrow`

**Root cause**: After certain expression types (hash/array dereference), the parser doesn't recognize `->` as a method call continuation.

### 3F. Complex List/Hash Construction in Args (~45 files)

**Error**: `expected expression, found Comma`

**Construct**: Multi-expression list arguments with mixed commas.

**After Wave 3**: measured after landing

---

## Wave 4 — Long Tail (~150 files)

| Category | Files | Example |
|----------|-------|---------|
| `return` in expression context | ~9 | `return $x if $cond` edge cases |
| `next`/`last` with complex expressions | ~10 | `next unless length $var` |
| `eval` block edge cases | ~5 | Nested eval with complex error handling |
| `goto` in expression context | ~3 | `goto &subroutine` |
| `RightBrace at Eof` (unclosed blocks) | ~30 | Cascade from earlier errors |
| Miscellaneous (each <5 files) | ~90 | Various rare constructs |

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

| Wave | Effort | Impact | Clean Rate |
|------|--------|--------|------------|
| 1 (done) | 4 merged PRs (#1215-#1218) | baseline | 51.1% |
| 2 (next) | 4 targeted fixes | +500 files | measured |
| 3 | 6 expression fixes | +300 files | measured |
| 4 | Long tail cleanup | +150 files | measured |

Wave 2 is the sweet spot: 4 fixes for ~500 files, with item 2A alone worth 261 files.

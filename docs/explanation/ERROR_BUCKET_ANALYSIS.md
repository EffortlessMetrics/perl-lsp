# Error Bucket Analysis

This document explains the error bucket methodology used in the parser corpus sweep. It covers why we analyze only the first error per file, how raw error messages are normalized into semantic buckets, and how those buckets drive parser improvement priorities.

## Overview

When the v3 recursive descent parser encounters a construct it cannot handle, it produces an `Error` AST node containing a diagnostic message. A single misparse typically cascades into 10-20 additional `Error` nodes downstream as the parser struggles to resynchronize. The corpus sweep (run via `just corpus-sweep`) parses every `.pm` file in the system Perl installation and categorizes the **first** error in each failing file into a semantic bucket.

The result is a concise map from root-cause categories to file counts, telling us exactly where parser improvements will have the largest impact.

> **Data sources**: This document references two corpora. The **system corpus** (7,095 files from `/usr/share/perl`, `/usr/lib/x86_64-linux-gnu/perl`, `/usr/share/perl5`) is tracked in the committed baseline at `.ci/parser-corpus-baseline.json`. The **CPAN corpus** (4,355 files from `target/cpan-corpus/lib/perl5`) is tracked in `.ci/cpan-corpus-baseline.json`. Both use the same first-error normalization pipeline, and every count in this document comes from those checked-in baselines.

## Why First-Error Analysis

Consider a file where the parser fails to recognize `split /regex/, $string`. That initial misparse (treating `/` as division) leaves the parser in a confused state, generating a chain of secondary errors: an unexpected token here, an unclosed paren there, a missing semicolon further down. Counting all errors would record perhaps 15 `ERROR` nodes, but only one of them is a root cause.

First-error analysis solves this by:

1. Walking the entire AST to find all `Error` nodes.
2. Selecting the one with the **smallest byte offset** (earliest in the source).
3. Normalizing that single error message into a semantic bucket.
4. Ignoring every downstream error in that file.

This gives a 1:1 mapping from failing file to root cause. For example, the original baseline (7,095 system files, 3,420 with errors) produced 3,420 first-error classifications spread across 25 semantic buckets, rather than 66,771 raw error nodes with significant duplication. After multiple waves of parser fixes, the current system baseline has 1,908 files with errors (28,383 error nodes) and the CPAN baseline has 1,212 files with errors (7,648 error nodes).

## The Normalization Pipeline

Raw error messages from the parser contain position information and varying phrasing. The `normalize_error_bucket()` function in `xtask/src/tasks/parser_corpus_sweep.rs` applies a two-pass normalization:

### Pass 1: Strip Position Information

Two regex patterns remove positional noise:

| Pattern | Example Input | Output |
|---------|---------------|--------|
| `^Invalid syntax at position \d+: (.+)$` | `Invalid syntax at position 1006: Potential catastrophic backtracking detected` | `Potential catastrophic backtracking detected` |
| ` at \d+$` | `expected RightBracket, found Eof at 42` | `expected RightBracket, found Eof` |

Both regexes are compiled once via `LazyLock<Option<Regex>>` and use `.ok()` for graceful degradation if regex compilation fails.

### Pass 2: Semantic Bucket Lookup

The position-stripped message is matched against the `SEMANTIC_BUCKETS` table using **substring containment** (first match wins). If no entry matches, the stripped message passes through verbatim as its own bucket name.

The first-match-wins ordering matters. For example, `"expected expression, found FatArrow"` matches the `unexpected_fat_arrow_expr` entry before it could fall through to the more general `unexpected_token_in_expr` entry. Similarly, `"expected RightBrace, found Semicolon"` matches `unclosed_brace_semicolon` before the generic `unclosed_brace`.

## The Semantic Bucket Table

The table below lists all semantic buckets defined in `SEMANTIC_BUCKETS`, plus the synthetic `catastrophic_parse_failure` bucket (used when the parser itself returns `Err`, e.g., recursion limit exceeded).

Two sets of counts are shown:
- **System** column: system Perl corpus baseline at commit `a44f7a6d` (2026-03-16, Perl 5.038002, 7,095 files, 5,139 clean = 72.4%)
- **CPAN** column: committed CPAN top-1000 baseline at commit `0ff44b44` (2026-03-17, 4,355 files, 3,139 clean = 72.1%)

The CPAN corpus numbers are the primary reference for current prioritization. Scout notes below are qualitative annotations only; the ranked counts and full tables all use the same committed baseline data.

### Top 9 Buckets by CPAN Baseline Count

| Rank | Bucket | CPAN | System | Root Cause Notes | Status |
|------|--------|------|--------|------------------|--------|
| 1 | `unclosed_paren_identifier` | 180 | 319 | Primary root cause: block-list functions called inside parenthesized expressions. When `map { ... }`, `grep { ... }`, or `sort { ... }` appear inside `(...)`, the parser misidentifies the block's closing `}` and then sees the next identifier where `)` was expected. | Active |
| 2 | `unexpected_token_in_expr` | 148 | 706 | Catch-all for unrecognized expression starts. Scout notes suggest the remaining files are a mix of subcategories that still need a fresh breakout pass. | Needs re-triage |
| 3 | `unclosed_paren` | 108 | 134 | Generic unclosed parenthesis. Mix of cascade errors and genuine misparses in complex nested expressions. | Active |
| 4 | `unexpected_question_expr` | 103 | 52 | Two confirmed root causes: (1) `use constant` with ternary -- `use constant FOO => $x ? 1 : 0` where the parser does not expect `?` after a constant-context expression; (2) named unary operators followed by ternary -- `-e $file ? \"yes\" : \"no\"` where the parser consumes the ternary `?` as part of the unary's argument. | Active |
| 5 | `unexpected_rbrace_expr` | 83 | -- | `}` found where an expression was expected. Typically occurs when the parser misidentifies a hash dereference block boundary or when a bare block ends in expression context. New bucket (not present in original doc). | Active |
| 6 | `unexpected_fat_arrow_expr` | 76 | 38 | `=>` used as a general separator (e.g., `push @arr => $val`). Wave 2B fixed several code paths, but additional separator-position call sites still remain. | Partially fixed |
| 7 | `expected_left_brace` | 68 | 54 | Missing `{` to open a block. This still mixes genuine missing-block cases with class/field-related block expectations. | Active |
| 8 | `unexpected_comma_expr` | 68 | -- | `,` found where an expression was expected. Common in list contexts where an empty element or trailing comma in a non-list position confuses the parser. New bucket (not present in original doc). | Active |
| 9 | `expected_comma_or_close_paren` | 55 | 11 | Argument list parsing failure where the parser loses track of list separators or closing delimiters. | Active |

### Expression Parsing Buckets (Full Table)

| Bucket | Trigger Substring | System | CPAN | Meaning |
|--------|-------------------|--------|------|---------|
| `unexpected_token_in_expr` | `expected expression, found` | 706 | 148 | Catch-all for expression-start failures not covered by specific buckets below |
| `unexpected_fat_arrow_expr` | `expected expression, found '=>'` | 38 | 76 | `=>` used where `,` would go (e.g., `push @arr => $val`); valid Perl, auto-quotes LHS |
| `unexpected_arrow_expr` | `expected expression, found '->'` | 145 | 14 | `->` method call continuation not recognized after certain expression types |
| `unexpected_slash_expr` | `expected expression, found '/'` | 2 | 6 | `/` treated as division when it should be a regex delimiter |
| `unexpected_question_expr` | `expected expression, found '?'` | 52 | 103 | `?` in ternary not recognized in certain complex expression contexts |
| `unexpected_return_expr` | `expected expression, found 'return'` | -- | -- | `return` in expression context (fixed in Wave 4A, PR #1483) |
| `unexpected_comma_expr` | `expected expression, found ','` | -- | 68 | `,` found where expression expected; empty list elements or misplaced commas |
| `unexpected_rbrace_expr` | `expected expression, found '}'` | -- | 83 | `}` found where expression expected; hash/block boundary confusion |
| `unexpected_rparen_expr` | `expected expression, found ')'` | -- | 2 | `)` found where expression expected |
| `unexpected_semicolon_expr` | `expected expression, found ';'` | -- | 14 | `;` found where expression expected |
| `unexpected_eof_expr` | `expected expression, found 'end of input'` | -- | -- | End of input where expression expected |
| `unexpected_word_op_or` | `expected expression, found 'or'` | -- | 26 | `or` found where expression expected |
| `unexpected_word_op_and` | `expected expression, found 'and'` | -- | 7 | `and` found where expression expected |
| `unexpected_word_op_not` | `expected expression, found 'not'` | -- | 8 | `not` found where expression expected |
| `unexpected_word_op_xor` | `expected expression, found 'xor'` | -- | -- | `xor` found where expression expected |
| `unexpected_token_unless` | `expected expression, found 'unless'` | -- | -- | Postfix `unless` misidentified as expression token |
| `unexpected_token_until` | `expected expression, found 'until'` | -- | -- | Postfix `until` misidentified as expression token |
| `unexpected_token_while` | `expected expression, found 'while'` | -- | -- | Postfix `while` misidentified as expression token |
| `unexpected_token_else` | `expected expression, found 'else'` | -- | 12 | `else` found where expression expected |
| `unexpected_token_elsif` | `expected expression, found 'elsif'` | -- | 8 | `elsif` found where expression expected |
| `unexpected_token_for` | `expected expression, found 'for'` | -- | -- | Postfix `for` misidentified as expression token |
| `unexpected_token_foreach` | `expected expression, found 'foreach'` | -- | -- | Postfix `foreach` misidentified as expression token |
| `unexpected_token_use` | `expected expression, found 'use'` | -- | -- | `use` found where expression expected |
| `unexpected_token_no` | `expected expression, found 'no'` | -- | -- | `no` found where expression expected |

### Delimiter Mismatch Buckets

| Bucket | Trigger Substring | System | CPAN | Meaning |
|--------|-------------------|--------|------|---------|
| `unclosed_bracket` | `expected ']'` | 16 | 38 | Array subscript `[...]` not properly closed |
| `unclosed_paren_identifier` | `expected ')', found identifier` | 319 | 180 | Closing `)` expected but found a bare identifier; primary root cause is block-list functions (`map`/`grep`/`sort`) inside parenthesized expressions |
| `unclosed_brace_semicolon` | `expected '}', found ';'` | 43 | 34 | Block `{...}` terminated by `;` instead of `}` |
| `unclosed_brace` | `expected '}'` | 50 | 30 | Generic unclosed brace (catch-all for brace mismatches) |
| `unclosed_paren` | `expected ')'` | 134 | 108 | Generic unclosed parenthesis |
| `unclosed_brace_eof` | `expected '}', found end of input` | -- | -- | File ends with unclosed block; usually cascade from an earlier misparse |
| `unclosed_angle` | `Expected '>' to close angle` | 2 | 8 | Unclosed angle bracket in diamond operator or `<FILEHANDLE>` |

### Expected Token Buckets

| Bucket | Trigger Substring | System | CPAN | Meaning |
|--------|-------------------|--------|------|---------|
| `expected_variable` | `Expected variable, found` | 178 | 8 | Parser expected a variable; system count inflated by `field` keyword regression |
| `expected_colon` | `expected ':'` | 22 | 26 | Missing `:` in ternary or label context |
| `expected_left_brace` | `expected '{'` | 54 | 68 | Missing `{` to open a block; partial regression from `field` keyword support |
| `expected_identifier` | `expected identifier` | 30 | 30 | Expected a bare identifier (subroutine name, label, etc.) |
| `expected_left_paren` | `expected '('` | 54 | 7 | Missing `(` where required by syntax |
| `expected_comma` | `expected ','` | 6 | 4 | Missing comma in list context |
| `expected_module_name` | `Expected module name or version` | 14 | -- | `use`/`require` statement with unrecognized module name syntax |
| `expected_semicolon` | `expected ';'` | 8 | 11 | Statement not properly terminated |
| `expected_comma_or_close_paren` | `Expected comma or closing parenthesis` | 11 | 55 | Argument list parsing failure (not in signature context) |
| `expected_import_item` | `Expected string or identifier in import` | 6 | 12 | Import list (`use Module qw(...)`) contains unexpected token |

### Special Buckets

| Bucket | Trigger Substring | System | CPAN | Meaning |
|--------|-------------------|--------|------|---------|
| `catastrophic_backtracking` | `catastrophic backtracking` | -- | -- | Regex engine safety guard (fixed in Wave 1) |
| `signature_param` | `Expected comma or closing parenthesis in signature` | 2 | -- | Subroutine signature parsing failure |
| `substitution_misparse` | `Substitution operator should be` | 2 | 10 | `s///` with unusual delimiters not recognized |

### Synthetic Bucket

| Bucket | Trigger | System | CPAN | Meaning |
|--------|---------|--------|------|---------|
| `catastrophic_parse_failure` | Parser returns `Err(...)` | 0 | 0 | Parser itself panicked or hit recursion limit. Ratchet enforces this stays at 0. |

### Unbucketed Errors

Both corpora contain a small number of errors that do not match any `SEMANTIC_BUCKETS` entry and pass through verbatim. These are individually rare (2-4 files each) and include position-specific messages that the normalization regexes did not fully strip. Examples from the CPAN baseline:

| Raw Message | CPAN Files |
|-------------|------------|
| `CHECK must be followed by a block` | 2 |
| `Expected comma or right parenthesis` | 2 |
| `Missing replacement in substitution` | 2 |
| `expected Comma, found Some(Identifier) at position 619` | 2 |

These are candidates for future bucket entries if their counts grow.

## Ordering Matters

The `SEMANTIC_BUCKETS` table is ordered from most-specific to most-general within each category. This is critical because the lookup uses first-match-wins:

```
"expected expression, found '=>'"       -->  unexpected_fat_arrow_expr
"expected expression, found '->'"       -->  unexpected_arrow_expr
"expected expression, found '/'"        -->  unexpected_slash_expr
"expected expression, found '?'"        -->  unexpected_question_expr
"expected expression, found 'return'"   -->  unexpected_return_expr
"expected expression, found 'unless'"   -->  unexpected_token_unless
  ... (other keyword/operator subcategories) ...
"expected expression, found ','"        -->  unexpected_comma_expr
"expected expression, found ';'"        -->  unexpected_semicolon_expr
"expected expression, found '}'"        -->  unexpected_rbrace_expr
"expected expression, found ')'"        -->  unexpected_rparen_expr
"expected expression, found"            -->  unexpected_token_in_expr   (catch-all)
```

If the catch-all `"expected expression, found"` appeared first, every subcategory error would be swallowed into the generic bucket and the roadmap could not distinguish them.

The same pattern applies to brace errors:

```
"expected '}', found ';'"              -->  unclosed_brace_semicolon
"expected '}', found end of input"     -->  unclosed_brace_eof
"expected '}'"                         -->  unclosed_brace             (catch-all)
```

## How Buckets Drive Priorities

### Largest Bucket = Largest Fix

The roadmap in `docs/project/PARSER_EDGE_CASE_ROADMAP.md` orders work by bucket size. After Waves 1-2 landed (and many original buckets shrank dramatically), the current CPAN corpus priority order is:

| Priority | Bucket(s) | CPAN Files | Root Cause | Status |
|----------|-----------|------------|------------|--------|
| Next | `unclosed_paren_identifier` | 180 | Block-list functions in parens (`map`/`grep`/`sort`) | Active |
| Next | `unexpected_token_in_expr` | 148 | Catch-all bucket that still needs re-triage | Needs re-triage |
| Next | `unclosed_paren` | 108 | Generic unclosed parenthesis | Active |
| Next | `unexpected_question_expr` | 103 | `use constant` ternary + named unary ternary | Active |
| Next | `unexpected_rbrace_expr` | 83 | Hash/block boundary confusion | Active |
| Next | `unexpected_fat_arrow_expr` | 76 | Additional `=>` separator sites | Partially fixed |
| Next | `expected_left_brace` | 68 | Missing-block and class/field block-expectation mix | Active |
| Next | `unexpected_comma_expr` | 68 | Empty list elements / misplaced commas | Active |
| Next | `expected_comma_or_close_paren` | 55 | Argument-list delimiter recovery gaps | Active |

The largest single-root-cause win in the committed CPAN baseline is now `unclosed_paren_identifier` (180 files), where fixing block-list function parsing inside parenthesized expressions would address the biggest reproducible bucket. The `unexpected_token_in_expr` catch-all at 148 is still close behind, but it remains a re-triage target rather than a clearly isolated fix class.

### Known Root Causes (2026-03-17 Baselines + 2026-03-18 Scout Notes)

The following root causes have been identified through scout analysis of the CPAN corpus:

**`unclosed_paren_identifier` (180 CPAN files)** -- Block-list functions (`map`, `grep`, `sort`) called inside parenthesized expressions. When the parser encounters `for my $x (map { BLOCK } @list)`, it misidentifies the block boundary and then sees an identifier where `)` was expected. The fix requires teaching the expression parser to recognize block-list function calls as valid subexpressions within parenthesized contexts.

**`unexpected_question_expr` (103 CPAN files)** -- Two distinct root causes confirmed:
1. *`use constant` with ternary*: `use constant FOO => $x ? 1 : 0` -- the parser does not expect `?` after the expression in a constant declaration context.
2. *Named unary operators followed by ternary*: `-e $file ? "yes" : "no"` -- the parser consumes the ternary `?` as part of the unary operator's argument rather than recognizing it as a separate binary operator.

**`unexpected_fat_arrow_expr` (76 CPAN files)** -- Wave 2B fixed the highest-volume `=>` separator paths, but the committed CPAN baseline shows the parser still misses additional call sites where `=>` appears in separator position outside canonical hash constructors.

**`expected_left_brace` (68 CPAN / 54 system)** -- This bucket mixes genuine missing-block cases with class/field-related block expectations. It remains worth triaging, but the committed CPAN baseline does not support treating it as a standalone `expected_variable`-style CPAN regression.

**`expected_comma_or_close_paren` (55 CPAN files)** -- The parser still loses track of separators or closing delimiters in some argument-list contexts. This is now large enough in the committed CPAN baseline to rank alongside the more obvious expression buckets.

**`unexpected_token_in_expr` (148 CPAN files)** -- This remains a heterogeneous catch-all bucket. Scout notes indicate many of these files likely belong in more-specific subcategories, so the next step here is re-triage rather than treating the bucket itself as a single root cause.

### Cascade Unmasking

When a bucket is fixed, files that previously failed at that point may now parse further and fail at a different construct. This "unmasks" errors that were previously hidden behind the first error. In practice:

- Fixing bucket A (500 files) does not always yield 500 clean files.
- Some files gain a new first-error in bucket B or a previously-unseen bucket C.
- New buckets are allowed by the ratchet (they indicate progress, not regression).

This is why the roadmap lists "measured" for post-wave clean rates rather than predicted numbers.

## Ratchet Enforcement

The corpus sweep enforces a **multi-metric ratchet** when run with `--enforce --baseline .ci/parser-corpus-baseline.json`. This prevents regressions across five dimensions:

| Metric | Rule | Rationale |
|--------|------|-----------|
| `crash_count` | Must be 0 | Parser must never crash (`catastrophic_parse_failure`) |
| `files_unreadable` | Must not increase | Encoding handling must not regress |
| `clean_files` | Must not decrease | Overall progress must be monotonic |
| `total_error_nodes` | Must not increase | Even cascade errors must not grow |
| Per-bucket counts | Each must not increase | No bucket may regress independently |

New buckets (not present in the baseline) are explicitly allowed. When a fix unmasks errors that normalize to a bucket name not in the baseline, the ratchet does not flag it. This prevents false positives from cascade unmasking.

### Enforcement Modes

| Mode | Trigger | Policy |
|------|---------|--------|
| System corpus | `just corpus-sweep-check` | Multi-metric ratchet against `.ci/parser-corpus-baseline.json` |
| Common corpus | `--manifest` flag | Strict zero-error policy (all listed modules must parse cleanly) |

## Updating the Baseline

After landing parser improvements:

```bash
# Run sweep and generate new baseline
just corpus-sweep-update

# Verify the new baseline passes ratchet against itself
just corpus-sweep-check

# Commit the updated baseline
git add .ci/parser-corpus-baseline.json
```

The baseline at `.ci/parser-corpus-baseline.json` is the single source of truth for corpus health. It is schema-versioned (currently `1.1.0`) and records the commit hash, timestamp, Perl version, and full bucket breakdown.

## Adding New Buckets

When the parser produces a new error message pattern that appears in multiple files:

1. Add a `(substring, bucket_name)` entry to `SEMANTIC_BUCKETS` in `xtask/src/tasks/parser_corpus_sweep.rs`.
2. Place it **before** any more-general entry that would match the same substring (first-match-wins).
3. Run `cargo test -p xtask` to verify the mapping (the `test_normalize_error_bucket_all_semantic_buckets_reachable` test ensures every entry can be triggered).
4. Run `just corpus-sweep-update` to regenerate the baseline with the new bucket broken out.

Without a dedicated bucket entry, new error patterns pass through verbatim as their own bucket names. This is intentional -- it surfaces novel errors in the sweep output so they can be triaged and, if common enough, given a proper bucket.

The bucket table has grown significantly since the original 25 entries. As of 2026-03-18, `SEMANTIC_BUCKETS` contains entries for keyword tokens (`unless`, `until`, `while`, `else`, `elsif`, `for`, `foreach`, `use`, `no`), word operators (`or`, `and`, `not`, `xor`), and punctuation subcategories (`,`, `;`, `}`, `)`, `end of input`) -- all broken out from the original `unexpected_token_in_expr` catch-all to enable finer-grained prioritization.

## Key Files

| File | Purpose |
|------|---------|
| `xtask/src/tasks/parser_corpus_sweep.rs` | Sweep implementation, `SEMANTIC_BUCKETS` table, `normalize_error_bucket()` |
| `.ci/parser-corpus-baseline.json` | System Perl corpus baseline with per-bucket counts |
| `.ci/cpan-corpus-baseline.json` | CPAN top-1000 corpus baseline with per-bucket counts |
| `docs/project/PARSER_EDGE_CASE_ROADMAP.md` | Fix waves organized by bucket priority |

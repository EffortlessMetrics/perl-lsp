# Error Bucket Analysis

This document explains the error bucket methodology used in the parser corpus sweep. It covers why we analyze only the first error per file, how raw error messages are normalized into semantic buckets, and how those buckets drive parser improvement priorities.

## Overview

When the v3 recursive descent parser encounters a construct it cannot handle, it produces an `Error` AST node containing a diagnostic message. A single misparse typically cascades into 10-20 additional `Error` nodes downstream as the parser struggles to resynchronize. The corpus sweep (run via `just corpus-sweep`) parses every `.pm` file in the system Perl installation and categorizes the **first** error in each failing file into a semantic bucket.

The result is a concise map from root-cause categories to file counts, telling us exactly where parser improvements will have the largest impact.

## Why First-Error Analysis

Consider a file where the parser fails to recognize `split /regex/, $string`. That initial misparse (treating `/` as division) leaves the parser in a confused state, generating a chain of secondary errors: an unexpected token here, an unclosed paren there, a missing semicolon further down. Counting all errors would record perhaps 15 `ERROR` nodes, but only one of them is a root cause.

First-error analysis solves this by:

1. Walking the entire AST to find all `Error` nodes.
2. Selecting the one with the **smallest byte offset** (earliest in the source).
3. Normalizing that single error message into a semantic bucket.
4. Ignoring every downstream error in that file.

This gives a 1:1 mapping from failing file to root cause. The baseline (7,095 files, 3,420 with errors) produces 3,420 first-error classifications spread across 25 semantic buckets, rather than 66,771 raw error nodes with significant duplication.

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

The table below lists all 25 semantic buckets defined in `SEMANTIC_BUCKETS`, plus the synthetic `catastrophic_parse_failure` bucket (used when the parser itself returns `Err`, e.g., recursion limit exceeded). Counts are from the baseline at commit `1429978a` (2026-03-09, Perl 5.038002, 7,095 files).

### Expression Parsing Buckets

| Bucket | Trigger Substring | Files | Meaning | Roadmap |
|--------|-------------------|-------|---------|---------|
| `unexpected_token_in_expr` | `expected expression, found` | 596 | Parser expected an expression but found an unexpected token; catch-all for expression-start failures not covered by specific buckets below | Waves 3-4 |
| `unexpected_fat_arrow_expr` | `expected expression, found FatArrow` | 310 | `=>` used where `,` would go (e.g., `push @arr => $val`); valid Perl, auto-quotes LHS | Wave 2B |
| `unexpected_arrow_expr` | `expected expression, found Arrow` | 142 | `->` method call continuation not recognized after certain expression types (hash/array deref) | Wave 3E |
| `unexpected_slash_expr` | `expected expression, found Slash` | 105 | `/` treated as division when it should be a regex delimiter (e.g., `split /pattern/`) | Wave 2C |
| `unexpected_question_expr` | `expected expression, found Question` | 30 | `?` in ternary not recognized in certain complex expression contexts | Wave 3C |
| `unexpected_return_expr` | `expected expression, found Return` | 30 | `return` in expression context edge cases | Wave 4 |

### Delimiter Mismatch Buckets

| Bucket | Trigger Substring | Files | Meaning | Roadmap |
|--------|-------------------|-------|---------|---------|
| `unclosed_bracket` | `expected RightBracket, found` | 544 | Array subscript `[...]` not properly closed; largely from package-qualified variables like `$Pkg::Var[idx]` | Wave 2A |
| `unclosed_paren_identifier` | `expected RightParen, found Identifier` | 488 | Closing `)` expected but found a bare identifier; often from `map`/`grep` blocks inside `for` iterators | Wave 3A/3B |
| `unclosed_brace_semicolon` | `expected RightBrace, found Semicolon` | 446 | Block `{...}` terminated by `;` instead of `}`; typically cascade from misparse of statement modifiers | Wave 2D |
| `unclosed_brace` | `expected RightBrace, found` | 187 | Generic unclosed brace (catch-all for brace mismatches not covered by specific sub-buckets) | Waves 2-4 |
| `unclosed_paren` | `expected RightParen, found` | 102 | Generic unclosed parenthesis | Waves 3-4 |
| `unclosed_brace_eof` | `expected RightBrace, found Eof` | 52 | File ends with unclosed block; usually cascade from an earlier misparse | Wave 4 |
| `unclosed_angle` | `Expected '>' to close angle` | 2 | Unclosed angle bracket in diamond operator or `<FILEHANDLE>` | -- |

### Expected Token Buckets

| Bucket | Trigger Substring | Files | Meaning | Roadmap |
|--------|-------------------|-------|---------|---------|
| `expected_variable` | `Expected variable, found` | 128 | Parser expected a variable (`$x`, `@a`, `%h`) but found something else; common with complex dereferences | Waves 2-3 |
| `expected_colon` | `expected Colon, found` | 32 | Missing `:` in ternary or label context | Wave 3C |
| `expected_left_brace` | `expected LeftBrace, found` | 28 | Missing `{` to open a block (e.g., after `sub name`) | Waves 3-4 |
| `expected_identifier` | `expected Identifier, found` | 20 | Expected a bare identifier (subroutine name, label, etc.) | Waves 3-4 |
| `expected_left_paren` | `expected LeftParen, found` | 20 | Missing `(` where required by syntax | Waves 3-4 |
| `expected_comma` | `expected Comma, found` | 12 | Missing comma in list context | Wave 3F |
| `expected_module_name` | `Expected module name or version` | 10 | `use`/`require` statement with unrecognized module name syntax | -- |
| `expected_semicolon` | `expected Semicolon, found` | 8 | Statement not properly terminated | Waves 3-4 |
| `expected_comma_or_close_paren` | `Expected comma or closing parenthesis` | 7 | Argument list parsing failure (not in signature context) | Waves 3-4 |
| `expected_import_item` | `Expected string or identifier in import` | 6 | Import list (`use Module qw(...)`) contains unexpected token | -- |

### Special Buckets

| Bucket | Trigger Substring | Files | Meaning | Roadmap |
|--------|-------------------|-------|---------|---------|
| `catastrophic_backtracking` | `catastrophic backtracking` | 111 | Regex engine safety guard fired; parser's internal regex for a construct hit exponential behavior | Wave 1 (partially fixed) |
| `signature_param` | `Expected comma or closing parenthesis in signature` | 2 | Subroutine signature `sub foo($x, $y)` parsing failure | -- |
| `substitution_misparse` | `Substitution operator should be` | 2 | `s///` substitution with unusual delimiters not recognized | -- |

### Synthetic Bucket

| Bucket | Trigger | Files | Meaning |
|--------|---------|-------|---------|
| `catastrophic_parse_failure` | Parser returns `Err(...)` | 0 (baseline) | Parser itself panicked or hit recursion limit; not an AST error node but a total failure. Ratchet enforces this stays at 0. |

## Ordering Matters

The `SEMANTIC_BUCKETS` table is ordered from most-specific to most-general within each category. This is critical because the lookup uses first-match-wins:

```
"expected expression, found FatArrow"   -->  unexpected_fat_arrow_expr
"expected expression, found Arrow"      -->  unexpected_arrow_expr
"expected expression, found Slash"      -->  unexpected_slash_expr
"expected expression, found Question"   -->  unexpected_question_expr
"expected expression, found Return"     -->  unexpected_return_expr
"expected expression, found"            -->  unexpected_token_in_expr   (catch-all)
```

If the catch-all `"expected expression, found"` appeared first, every `FatArrow`/`Arrow`/`Slash` error would be swallowed into the generic bucket and the roadmap could not distinguish them.

The same pattern applies to brace errors:

```
"expected RightBrace, found Semicolon"  -->  unclosed_brace_semicolon
"expected RightBrace, found Eof"        -->  unclosed_brace_eof
"expected RightBrace, found"            -->  unclosed_brace             (catch-all)
```

## How Buckets Drive Priorities

### Largest Bucket = Largest Fix

The roadmap in `docs/project/PARSER_EDGE_CASE_ROADMAP.md` orders work by bucket size:

| Priority | Bucket(s) | Files | Fix |
|----------|-----------|-------|-----|
| Wave 2A | `unclosed_bracket` | 544 | Package-qualified array subscript (`$Pkg::Var[idx]`) |
| Wave 2B | `unexpected_fat_arrow_expr` | 310 | `=>` as general separator (`push @a => $v`) |
| Wave 2C | `unexpected_slash_expr` | 105 | `split /regex/` slash disambiguation |
| Wave 2D | `unclosed_brace_semicolon` | 446 | Statement modifiers after complex expressions |

A single fix for Wave 2A (package-qualified subscripts) addresses 544 files -- the largest single-fix win in the entire corpus.

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
3. Run `cargo test -p xtask` to verify the mapping.
4. Run `just corpus-sweep-update` to regenerate the baseline with the new bucket broken out.

Without a dedicated bucket entry, new error patterns pass through verbatim as their own bucket names. This is intentional -- it surfaces novel errors in the sweep output so they can be triaged and, if common enough, given a proper bucket.

## Key Files

| File | Purpose |
|------|---------|
| `xtask/src/tasks/parser_corpus_sweep.rs` | Sweep implementation, `SEMANTIC_BUCKETS` table, `normalize_error_bucket()` |
| `.ci/parser-corpus-baseline.json` | Committed baseline with per-bucket counts |
| `docs/project/PARSER_EDGE_CASE_ROADMAP.md` | Fix waves organized by bucket priority |

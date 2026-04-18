# Verification Findings — work-b4f6edfe

## Confidence Assessment
**[MEDIUM]** — I independently verified the corpus baseline structure, parser error emission mechanism, and tested specific Perl patterns. However, I cannot fully reproduce the corpus sweep because the Perl 5.38.2 and CPAN corpus files don't exist in this environment. The baseline was built on a different system with a different Perl installation.

## Confirmed Findings

### Finding 1: Bucket structure is accurate
The `.ci/parser-corpus-baseline.json` correctly shows 26 entries in the `unexpected_token_in_expr` bucket, corresponding to 13 unique files × 2 Perl versions (5.38 and 5.38.2). Verified via direct inspection of the JSON structure:
- Corpus roots include `/usr/share/perl`, `/usr/lib/x86_64-linux-gnu/perl`, `/usr/share/perl5`
- `first_error_buckets` correctly maps `unexpected_token_in_expr` to 26 entries
- `files_by_bucket['unexpected_token_in_expr']` lists all 26 file entries

### Finding 2: Error emission mechanism confirmed
The `unexpected_token_in_expr` bucket is the catch-all for `ParseError::unexpected("expression", ...)` at `primary.rs:928`. The SEMANTIC_BUCKETS table in `xtask/src/tasks/parser_corpus_sweep.rs:100` maps the substring `"expected expression, found"` to this bucket. The ordering is correct — more specific buckets (e.g., `unexpected_fat_arrow_expr`, `unexpected_question_expr`) precede it so they match first.

### Finding 3: Prior art test file is real and passing
The `crates/perl-parser-core/tests/fix_unexpected_token_in_expr_2731.rs` contains 57 tests covering 4 sub-patterns (A-D):
- Pattern A: No-arg builtins before `&&`/`||`
- Pattern B: Special variable `$:`
- Pattern C: Typeglob with `*^N`, `*-{ARRAY}`, `*+{ARRAY}` suffixes
- Pattern D: `__END__`/`__DATA__` after no-semicolon statements
All 57 tests pass in the current codebase.

### Finding 4: CPAN files missing from this environment (correctly identified by research agent)
The 7 CPAN modules at `/usr/share/perl5/*` (MimeInfo, Run3, ToUnicode, Lite, Type, Sendmail, Writer) do NOT exist in this verification environment. The baseline was built on a system with these files installed. This is a real constraint on verification scope.

### Finding 5: Perl 5.38.2 also missing from this environment (correctly identified)
`/usr/share/perl/5.38.2/` does not exist. Only Perl 5.38 files are accessible for direct inspection.

## Corrected Findings

### Finding 6: Research Agent's Finding 2 is INCORRECT — `*+;` parses cleanly
The research agent claimed that `*LAST_PAREN_MATCH = *+;` (bare `*+` typeglob without `{ARRAY}` subscript) is an UNCOVERED pattern causing `unexpected_token_in_expr`. **This is wrong.**

I created a temporary test (`test_bare_plus_glob_temp.rs`) and verified that `*LAST_PAREN_MATCH = *+;` parses CLEANLY — it produces NO error nodes.

The actual failing patterns in English.pm are:
- `*PREMATCH = *\`;` — backtick typeglob (`$``) — **FAILS** with "expected expression, found unknown token"
- `*POSTMATCH = *';` — tick typeglob (`$'`) — **FAILS** with "expected expression, found unknown token"

These are typeglobs with the special punctuation variables `$`` and `$'`. The lexer does not recognize `` ` `` and `'` as valid typeglob suffix characters. This is a NEW sub-pattern distinct from the existing Pattern C (`*^N`, `*-{ARRAY}`, `*+{ARRAY}`).

### Finding 7: Research Agent's Finding 3 is INCORRECT — File/Copy.pm does NOT use indirect object notation
The research agent claimed File/Copy.pm "has `croak(...)` calls used as indirect objects (e.g., after a method call like `$fh->print`)". **This is imprecise/wrong.**

I inspected File/Copy.pm lines 78-95 and found `UNIVERSAL::isa(...)` calls, which are explicit function calls with arguments, NOT indirect object notation. I verified that `UNIVERSAL::isa($from, 'GLOB');` and `ref($from) eq 'GLOB' || UNIVERSAL::isa($from, 'GLOB');` both parse CLEANLY.

The actual trigger for File/Copy.pm's `unexpected_token_in_expr` error remains UNKNOWN — it could be a different construct in the file, or the baseline may reflect a version of the file with patterns now fixed in the current codebase.

### Finding 8: Research Agent's Finding 4 is INCORRECT — `BEGIN { eval {} }` parses cleanly
The research agent claimed that `BEGIN { eval { ... } }` "may confuse the statement/expression boundary parsing." **This is wrong for the specific pattern in IPC/Cmd.pm.**

I verified that `BEGIN { eval { require POSIX; }; }` and `BEGIN { use constant IS_VMS => 0; use Exporter; eval { require POSIX; }; }` both parse CLEANLY. The IPC/Cmd.pm BEGIN block contains simple `eval {}` blocks (not deeply nested), which the parser handles correctly.

The actual trigger for IPC/Cmd.pm's `unexpected_token_in_expr` error remains UNKNOWN.

## New Findings

### Finding 9: NEW sub-pattern identified — typeglob with backtick/tick suffixes
The most important new finding is that typeglob expressions with `` ` `` (backtick) and `'` (tick) suffixes are NOT handled by the parser. This is distinct from the already-fixed Pattern C (`*^N`, `*-{ARRAY}`, `*+{ARRAY}`). The error message is "expected expression, found unknown token", which maps to `unexpected_token_in_expr` via the catch-all bucket.

Specifically:
- `*PREMATCH = *\`;` — FAILS
- `*POSTMATCH = *';` — FAILS

These appear in English.pm and are real constructs: `$`` is the prematch variable and `$'` is the postmatch variable in Perl's regex match results. The typeglob form `*\`` and `*'` should alias these variables.

### Finding 10: Working typeglob patterns (English.pm)
The following typeglob patterns from English.pm parse CLEANLY in the current codebase (already handled by existing Pattern C or other parser code):
- `*LAST_PAREN_MATCH = *+;` — bare `*+` (research agent claimed this would fail — it doesn't)
- `*LAST_SUBMATCH_RESULT = *^N;` — `*^N` (already in tests)
- `*INPUT_LINE_NUMBER = *.;` — dot typeglob (works)
- `*MATCH = *&;` — ampersand typeglob (works)
- `*INPUT_RECORD_SEPARATOR = */;` — slash typeglob (works)

### Finding 11: Multiple patterns per file likely
English.pm contains BOTH working patterns (`*^N`, `*+`, `*.`, `*&`, `*/`) AND failing patterns (`*\``, `*'`). This means the file could produce errors from multiple locations, potentially explaining why the research agent's simplified analysis attributed it to a single cause.

## Scope Assessment

The issue title says "parser: fix unexpected_token_in_expr bucket — 26 CPAN corpus files" — this is accurate. The scope covers the 26 files in the `unexpected_token_in_expr` bucket across the corpus baseline.

However, the issue description's characterization of which patterns cause the errors in which files is INCOMPLETE and PARTIALLY INCORRECT:
- English.pm: Error is from `*\`` and `*'` typeglob suffixes, NOT from `*+` (which works)
- File/Copy.pm: Error trigger is UNKNOWN (not indirect objects or `UNIVERSAL::isa`)
- IPC/Cmd.pm: Error trigger is UNKNOWN (not the `BEGIN { eval {} }` pattern)

## Verification Methodology

### Step 1: Baseline inspection
- Loaded `.ci/parser-corpus-baseline.json` and enumerated all 26 `unexpected_token_in_expr` entries
- Verified 13 unique files × 2 Perl version entries
- Confirmed corpus roots and schema version

### Step 2: Codebase inspection
- Examined `primary.rs:925-929` to confirm error emission point
- Examined `xtask/src/tasks/parser_corpus_sweep.rs:50-100` to confirm SEMANTIC_BUCKETS mapping
- Examined `fix_unexpected_token_in_expr_2731.rs` to confirm existing pattern coverage (57 tests, all passing)

### Step 3: Environment accessibility check
- Verified Perl 5.38 files exist: English.pm, File/Copy.pm, IPC/Cmd.pm
- Verified Perl 5.38.2 files do NOT exist in this environment
- Verified CPAN files at `/usr/share/perl5/*` do NOT exist

### Step 4: Pattern verification via Rust tests
I wrote temporary Rust test files using `cpan_test_helpers::assert_clean_parse` and ran them via `cargo test`:

**Test results:**
- `*LAST_PAREN_MATCH = *+;` → PASSES (research agent claimed it fails)
- `*PREMATCH = *\`;` → FAILS (new sub-pattern found)
- `*POSTMATCH = *';` → FAILS (new sub-pattern found)
- `*INPUT_LINE_NUMBER = *.;` → PASSES
- `*MATCH = *&;` → PASSES
- `*INPUT_RECORD_SEPARATOR = */;` → PASSES
- `UNIVERSAL::isa(...)` → PASSES (research agent blamed indirect objects)
- `BEGIN { eval { ... } }` → PASSES (research agent blamed this pattern)

## Critical Unknowns

1. **What specifically triggers the errors in File/Copy.pm and IPC/Cmd.pm?** I tested the patterns I could identify from file inspection, but the actual error trigger remains unknown. It could be a construct I haven't identified, or the baseline may reflect a different version of these files.

2. **Are the 5.38.2 and CPAN corpus files still producing errors in the current codebase?** Since these files don't exist in this environment, I cannot verify whether the errors are still present or have been fixed in subsequent parser updates.

3. **Do the failing `*\`` and `*'` patterns fully account for English.pm's error?** Since I can't run the full corpus sweep on English.pm, I cannot confirm whether these are the ONLY failing patterns in that file.

## Summary

The research agent correctly identified the bucket size (26 entries, 13 unique files) and the high-level approach (decompose into sub-patterns). However, the agent's specific claims about which Perl patterns cause errors in which files are PARTIALLY INCORRECT. The only confirmed NEW sub-pattern is:
- **Typeglob with `` ` `` (backtick) and `'` (tick) suffixes** — `*\``, `*'` not recognized as valid expression starts

The existing Pattern C in `fix_unexpected_token_in_expr_2731.rs` already covers `*^N`, `*-{ARRAY}`, `*+{ARRAY}`, but the tick/backtick variants are a distinct gap.

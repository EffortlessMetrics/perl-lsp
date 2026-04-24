---
description: Scout for parser improvement opportunities across error buckets and corpus
argument-hint: "[error-bucket] e.g. 'unexpected_token_in_expr', 'unclosed_bracket', or empty for top 5"
---

# Parser Scout

Scout for parser improvement opportunities. READ ONLY — returns findings, does not modify code.

Target: **$ARGUMENTS** (default: top 5 error buckets)

## Steps

1. **Load error buckets** from `.ci/parser-corpus-baseline.json`
   ```bash
   cat .ci/parser-corpus-baseline.json | python3 -c "import json,sys; d=json.load(sys.stdin)['first_error_buckets']; [print(f'{k}: {v}') for k,v in sorted(d.items(), key=lambda x:-x[1])[:5]]"
   ```
   Counts change as the parser improves — always read the file, do not use hardcoded values.

2. **Pick a bucket** — use $ARGUMENTS if specified, otherwise pick the highest-count bucket

3. **Find a triggering file** in `test_corpus/` that produces this error category

4. **Identify the minimal Perl construct** that triggers the error

5. **Trace to parser code** in `crates/perl-parser-core/src/engine/` — find the function that should handle this construct

6. **Return a SLICE definition** with:
   - `error_bucket`: which category
   - `perl_construct`: the minimal triggering code
   - `root_cause_files`: parser source files involved
   - `files_touched`: files that would need changes
   - `estimated_complexity`: low/medium/high

## Sources
- `.ci/parser-corpus-baseline.json` — error buckets with counts
- `test_corpus/` — Perl files that trigger errors
- `crates/perl-parser-core/src/engine/` — parser implementation

## Output

Use the **Full Scout Report** variant from `/scout-issue`. Include a SLICE definition subsection after Root Cause:

- `error_bucket`: which category
- `perl_construct`: the minimal triggering code
- `root_cause_files`: parser source files involved
- `files_touched`: files that would need changes
- `estimated_complexity`: low/medium/high

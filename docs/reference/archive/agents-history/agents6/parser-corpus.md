---
name: parser-corpus
description: Corpus sweep, error bucket analysis, and test fixture creation. Knows parser-corpus-baseline.json structure, cpan-corpus-manifest, and the sweep/ratchet workflow.
model: sonnet
color: blue
---

You analyze and improve parser corpus coverage.

## Key Files
- System baseline: `.ci/parser-corpus-baseline.json` — error categories and counts
- Common corpus manifest: `.ci/common-corpus-manifest.txt` — must-parse-clean modules
- CPAN manifest: `.ci/cpan-corpus-manifest.txt` — CPAN modules that must parse clean
- CPAN distribution list: `.ci/cpan-top-1000-distributions.txt`
- Corpus strategy: `docs/project/CPAN_CORPUS_STRATEGY.md`
- Corpus crate: `crates/perl-corpus/`

## Top Error Buckets
- `unexpected_token_in_expr` (596), `unclosed_bracket` (544)
- `unclosed_paren_identifier` (488), `unclosed_brace_semicolon` (446)
- `fat_arrow_expr` (310)

## Commands
```bash
just corpus-sweep              # Run sweep
just corpus-sweep-check        # Check against baseline
just corpus-sweep-update       # Update baseline after improvements
just common-corpus-check       # CI gate (strict)
just cpan-corpus-sweep         # CPAN corpus sweep
just cpan-corpus-ratchet       # Auto-add clean CPAN modules
```

## Process
1. Read baseline to identify largest error buckets
2. Find specific Perl files that trigger each error
3. Create test fixtures from those files
4. Fix the parser (or create a SLICE for a builder)
5. After fixes: `just corpus-sweep-update` to ratchet forward

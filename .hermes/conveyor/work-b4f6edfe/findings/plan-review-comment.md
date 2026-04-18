# Plan Review — work-b4f6edfe

## Overview
This is the plan review for the `unexpected_token_in_expr` bucket decomposition work item. I reviewed the research agent's `initial_plan.md` against the actual codebase state and verification results.

## Risks Identified in the Plan

### Risk 1: Incorrect Pattern Attribution (CONFIRMED)
The plan assumes the research agent correctly identified the patterns causing errors in each file. My verification shows this is NOT the case:
- English.pm's error is from `*\`` and `*'` typeglob suffixes, NOT `*+` (which parses cleanly)
- File/Copy.pm's error trigger is UNKNOWN — not `UNIVERSAL::isa` or indirect object notation
- IPC/Cmd.pm's error trigger is UNKNOWN — not `BEGIN { eval {} }` (which parses cleanly)

**Impact**: The plan's Phase 1 ("extract error locations") assumes the wrong patterns are causing errors. This could lead to wasted effort extracting snippets of already-working code.

**Recommendation**: Before Phase 1 begins, the builder should run the corpus sweep or parse the actual failing files to identify the TRUE first-error locations, not rely on the research agent's attributions.

### Risk 2: Error Cascade Obscuring Root Cause (VALID)
The plan correctly identifies this risk. Since English.pm has BOTH working patterns (`*^N`, `*+`, etc.) AND failing patterns (`*\``, `*'`), fixing one may reveal another. The plan needs to account for this by suggesting iterative sweeps rather than one-shot fixes.

### Risk 3: CPAN Environment Gap (CONFIRMED)
The plan acknowledges this risk but understates it. The 7 CPAN modules and 3 Perl 5.38.2 files are NOT accessible in this environment. The builder can only directly inspect 3 of the 13 unique files. For the other 10 files, the builder must rely on:
1. Running a corpus sweep on a properly-equipped system
2. Inferring patterns from similar files in the CPAN corpus

**Impact**: The "produce follow-up issues" phase cannot be completed for 10 of 13 files without additional environment setup or corpus data.

### Risk 4: Multiple Sub-patterns Per File (CONFIRMED)
English.pm has at least TWO distinct failing patterns (`*\`` and `*'`), both from the same family (typeglob punctuation suffixes). If these are fixed separately, they should be tracked as separate sub-patterns even though they're related.

### Risk 5: Baseline Currency (NEW RISK)
The baseline was built on a different system. It's possible that some patterns in the baseline have since been fixed in the current codebase (e.g., the `*+` bare typeglob the research agent thought was broken actually parses cleanly now). The builder should verify against the current codebase, not trust the baseline blindly.

## Scope Concerns

### Scope: Correct Size, Incorrect Sub-pattern Identification
The issue title accurately describes 26 entries in the bucket across 13 files. However, the plan's file family grouping (English.pm, File/Copy.pm, IPC/Cmd.pm, CPAN) is based on incorrect pattern attribution. A revised grouping might look like:
- **English.pm family**: `*\`` and `*'` typeglob suffixes (new sub-pattern)
- **File/Copy.pm**: UNKNOWN — needs corpus sweep to identify
- **IPC/Cmd.pm**: UNKNOWN — needs corpus sweep to identify
- **CPAN modules**: UNKNOWN — files not accessible

## Specific Concerns with Plan's Phase 2

The plan says "For English.pm: confirm whether `*+;` (bare typeglob `+`) is a new sub-pattern". My verification shows `*+;` parses CLEANLY — it is NOT a new sub-pattern. The plan should have been more definitive: `*+;` is covered, but `*\`` and `*'` are NOT.

The plan also says "For File/Copy.pm: identify if indirect object or `UNIVERSAL::isa` is the trigger". My verification shows `UNIVERSAL::isa` parses cleanly. The plan should have been more definitive about what the actual trigger is.

## Recommendations for the Builder

1. **Run corpus sweep FIRST** before extracting snippets — the actual error locations must be verified, not inferred from file inspection alone
2. **Treat `*\`` and `*'` typeglob suffixes as a NEW sub-pattern** — file a targeted issue for these even though the plan was wrong about `*+`
3. **Expect iterative fixes** — for English.pm, expect at least 2 rounds of fixes (one for `*\``/`*'`, then re-evaluate)
4. **Do NOT blame `UNIVERSAL::isa` or `BEGIN { eval {} }`** for File/Copy.pm and IPC/Cmd.pm — these are red herrings

## Verdict

**The plan is PARTIALLY SOUND** but has critical errors in its pattern attribution that would lead the builder down wrong paths. The high-level approach (decompose bucket into sub-patterns with concrete test cases) is correct, but the specific sub-pattern identifications are wrong for at least 2 of the 3 accessible files.

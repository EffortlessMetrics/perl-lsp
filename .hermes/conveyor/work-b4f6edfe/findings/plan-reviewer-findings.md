# Plan Review Findings — work-b4f6edfe

## Overall Assessment
**feasible with modifications** — The high-level approach (decompose bucket into sub-patterns, file targeted issues) is correct, but the plan's specific pattern attributions are wrong for at least 2 of 3 accessible files, and the scope of "Phase 2" needs revision before this can proceed to builder.

## Scope Assessment

The issue title accurately describes 26 entries in the `unexpected_token_in_expr` bucket across 13 unique files. **However, the plan's file-family groupings are based on incorrect pattern attribution**:

| File Group | Plan's Claim | Verification Says |
|---|---|---|
| English.pm | `*+` bare typeglob causes error | `*+` parses CLEANLY; `*\`` and `*'` are the actual failing patterns |
| File/Copy.pm | Indirect objects / `UNIVERSAL::isa` | `UNIVERSAL::isa` parses CLEANLY; actual trigger UNKNOWN |
| IPC/Cmd.pm | `BEGIN { eval {} }` nesting | `BEGIN { eval {} }` parses CLEANLY; actual trigger UNKNOWN |

**Impact**: The plan's Phase 2 is largely invalidated — the "confirm whether X is the trigger" tasks would lead builders to verify patterns that are already working, not the actual failing ones.

## What Works

1. **High-level approach is correct**: Decomposing a catch-all error bucket into specific sub-patterns with concrete test cases is the established methodology for this project (proven by #2731's 4 sub-patterns).

2. **CORPUS_ROADMAP.md subcategory framework**: The 10-subcategory decomposition in the roadmap provides the right conceptual structure for categorizing new patterns.

3. **Verification agent corrected the record**: The verification agent identified that `*\`` and `*'` (backtick/tick typeglob suffixes) are the actual English.pm trigger — a genuinely new sub-pattern distinct from the already-fixed `*^N`, `*-{ARRAY}`, `*+{ARRAY}` patterns.

4. **Test file naming convention**: Using `fix_unexpected_token_in_expr_<issue>.rs` is established practice and avoids merge conflicts during simultaneous agent work.

5. **Risk identification**: The plan correctly identifies error cascade and multiple-patterns-per-file as risks. The CPAN environment gap is acknowledged.

## What Doesn't Work

### 1. Phase 2 is Built on Wrong Attributions
The plan tasks the builder to "confirm whether `*+;` is a new sub-pattern" and "identify if indirect object or `UNIVERSAL::isa` is the trigger." These verification tasks are backwards:
- `*+` already parses cleanly — no confirmation needed, this is a settled question
- The actual failing patterns (`*\``, `*'`) are already identified by the verification agent but NOT mentioned in the plan

**Consequence**: A builder following this plan would waste time verifying non-issues while the actual triggers remain uninvestigated.

### 2. Phase 1 Cannot Be Completed as Written
The plan says "run corpus sweep or manually parse English.pm, File/Copy.pm, IPC/Cmd.pm to find first-error offsets." The verification agent already found that:
- English.pm's failing patterns are `*\`` and `*'`
- File/Copy.pm and IPC/Cmd.pm triggers are UNKNOWN

The "manually parse" approach would require re-running the parser on these files with detailed error tracing — not just file inspection. The plan doesn't specify how to find the exact byte offset of the first error.

### 3. CPAN Files Cannot Be Investigated in This Environment
7 of 13 unique files are inaccessible. The plan acknowledges this but says they "need corpus extraction" — without specifying that the corpus sweep must be run on a system with those files present, not this one.

### 4. Baseline Currency
The baseline was built on commit `3f7ede36` at `2026-04-09`. It's possible that some patterns have since been partially fixed in the current codebase. The plan doesn't instruct the builder to verify the baseline against current code before beginning work.

## Top Risks

### Risk 1: Wrong Pattern Attribution Leads Builder Astray
- **Likelihood**: HIGH — the plan explicitly tasks the builder with verifying wrong patterns
- **Impact**: Builder spends Phase 2 confirming `*+` works (already known) while actual `*\``/`*'` patterns get no attention
- **Mitigation**: Replace Phase 2 classification tasks with verified findings from the verification agent:
  - English.pm: `*\`` and `*'` typeglob suffixes → NEW sub-pattern (file issue)
  - File/Copy.pm: trigger UNKNOWN → corpus sweep needed
  - IPC/Cmd.pm: trigger UNKNOWN → corpus sweep needed
  - CPAN modules (7 files): trigger UNKNOWN → cannot investigate in this environment

### Risk 2: Error Cascade Obscuring Root Cause (Validated)
- **Likelihood**: HIGH — the verification agent confirmed English.pm has BOTH working patterns (`*^N`, `*+`, `*-{ARRAY}`) AND failing patterns (`*\``, `*'`)
- **Impact**: Fixing `*\`` may reveal `*'` as a second distinct error in the same file; both need separate test cases and possibly separate issues
- **Mitigation**: Extract minimal snippets for EACH distinct pattern independently. Don't assume one fix solves the whole file.

### Risk 3: CPAN Environment Gap (Confirmed Unresolved)
- **Likelihood**: HIGH — 7 of 13 files are inaccessible, and the plan offers no path to investigate them
- **Impact**: "Produce follow-up issues" phase cannot be completed for >50% of the bucket entries
- **Mitigation**: Either (a) set up the corpus environment with the missing files before proceeding, or (b) explicitly scope this work item to only the 3 accessible files, with a separate follow-up item for the CPAN files.

### Risk 4: Baseline Regressions After Fix
- **Likelihood**: MEDIUM — the baseline is from a different commit and system
- **Impact**: A fix that "solves" one file might actually cause the baseline to regress on another file's pattern
- **Mitigation**: Run the full corpus sweep before and after each fix. Verify delta.

### Risk 5: Test Compilation Blocked
- **Likelihood**: HIGH — the verification agent noted `cargo test -p perl-parser-core` fails due to missing `insta` dev-dependency
- **Impact**: Builder cannot verify their test-driven fixes compile or pass
- **Mitigation**: Resolve the `insta` dependency issue before builder begins work, OR the builder must use `cargo build -p perl-parser-core --lib` only and skip the test-compilation step.

## Edge Cases

1. **Multiple distinct patterns in same file**: English.pm has `*\``, `*'`, `*^N`, `*-{ARRAY}`, `*+{ARRAY}` all in the same file. Fixing one may not reduce the bucket entry count if another pattern still fails.

2. **Perl version differences**: The 5.38 and 5.38.2 variants of the same file may have different error locations or patterns. The baseline shows both entries for each file, but they could have different root causes.

3. **CPAN corpus vs system corpus**: The issue references the system corpus baseline (`.ci/parser-corpus-baseline.json`), not the CPAN corpus. The CORPUS_ROADMAP.md discusses the CPAN corpus (4,355 files). These are different baselines with different error distributions.

4. **The baseline is a "first error" snapshot**: Each file's entry is its FIRST error only. A file with multiple errors will only show the first one. Fixing the first error may reveal a second error that was previously hidden — this is the cascade problem.

## Recommendations

1. **Replace Phase 2 with verified findings** — Remove the "confirm whether X triggers the error" tasks. Replace with the verification agent's findings:
   - English.pm: `*\`` and `*'` are the confirmed triggers → file issue immediately
   - File/Copy.pm: trigger UNKNOWN → document this, do not speculate
   - IPC/Cmd.pm: trigger UNKNOWN → document this, do not speculate

2. **Add corpus sweep step to Phase 1** — Before Phase 1, the builder should run the parser corpus sweep against the accessible files to get exact byte offsets for the first error in each file. File inspection alone is insufficient.

3. **Scope to accessible files only, or defer CPAN files** — Either explicitly scope this work item to the 3 accessible files (English.pm, File/Copy.pm, IPC/Cmd.pm) and file a separate work item for the 7 inaccessible CPAN files, OR set up the environment with the missing files before proceeding.

4. **Resolve `insta` dependency before builder starts** — The test compilation is blocked. Either fix the dependency or update the plan to work around it (e.g., builder verifies with `cargo build --lib` only, and a separate agent runs the tests).

5. **Add baseline verification step** — Before beginning, run the corpus sweep to confirm the 26-entry bucket still exists in the current codebase state. If it has already been partially fixed, update the plan accordingly.

6. **Treat `*\`` and `*'` as a single sub-pattern issue** — Both are typeglob punctuation suffixes. File ONE issue for "typeglob with `*\`` and `*'` suffixes" rather than two separate issues, since they likely share the same parser fix location.

## Confidence to Proceed

**medium** — The approach is sound and the methodology is proven, but the plan needs revision before it can guide a builder effectively. Specifically:

- The wrong patterns are being investigated (Phase 2)
- The inaccessible files cannot be handled in this environment (CPAN gap)
- The test infrastructure is blocked (`insta` dependency)

**What would raise confidence to high**:
1. Revise Phase 2 to use verified pattern findings instead of speculative ones
2. Explicitly scope this work to the 3 accessible files, deferring CPAN files
3. Resolve the `insta` dependency issue
4. Add baseline verification step before Phase 1

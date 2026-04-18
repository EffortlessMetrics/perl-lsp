# Maintainer Vision Findings — work-b4f6edfe

## Alignment Assessment
**ALIGNED** — The work of decomposing the `unexpected_token_in_expr` catch-all bucket into specific sub-patterns is precisely in line with the project's stated direction toward CPAN corpus confidence (ROADMAP.md Phase A/B) and the existing precedent set by #2731's decomposition methodology.

## Reasoning

### This Work Extends the Established Pattern
The CORPUS_ROADMAP.md explicitly identifies `unexpected_token_in_expr` as the largest error bucket (146 files in the CPAN corpus baseline, rank #1) and lists it as Phase A work. The project's Phase A strategy is: *"Fix the top 5 error buckets, each with a dedicated builder agent. Each builder receives a scout-produced spec with exact function names, line numbers, and CPAN file samples."*

This work-item does exactly that scouting/spec work for the `unexpected_token_in_expr` bucket — it is the prerequisite research that enables a builder to make targeted fixes. The existing `fix_unexpected_token_in_expr_2731.rs` file (57 tests, 4 sub-patterns fixed) is the proof-of-concept that this methodology works.

### It Aligns With the Parser Architecture's Error Taxonomy
The codebase deliberately categorizes parse errors into semantic buckets via `SEMANTIC_BUCKETS` in `xtask/src/tasks/parser_corpus_sweep.rs`. This design choice — making error buckets explicit, tracked, and decomposable — is a core architectural decision. The blockers.yaml even records `unexpected_token_in_expr` as a Tier 1 parser_blocker with status "fixed" via #2731, showing this is tracked work. The remaining 26 entries in the system corpus baseline are the tail that needs the same treatment.

### Correct Abstraction Level
This work decomposes at the right layer: it identifies sub-patterns in the failing code (typeglob suffixes `*` `` ` `` and `*'`, indirect object notation, BEGIN/eval nesting) and maps each to a specific parser location (e.g., `primary.rs:925` the `_` arm of `parse_primary_inner`). This is exactly what the CORPUS_ROADMAP calls for — concrete root-cause analysis, not surface-level workarounds.

### Connection to Project's Core Purpose
perl-lsp exists to provide IDE services for real Perl code. The affected files — English.pm, File/Copy.pm, IPC/Cmd.pm — are stdlib modules used by thousands of Perl developers. The 7 inaccessible CPAN files (MimeInfo, Run3, ToUnicode, Lite, Type, Sendmail, Writer) represent real-world code the parser needs to handle. The work serves the LSP's core contract: "your code should parse cleanly in our editor."

## Impact on Codebase Trajectory

**If merged as proposed**: The work produces 1-5 follow-up issues, each with minimal test cases and root-cause analysis. A future builder can pick up each issue and fix one sub-pattern at a time, adding regression tests to `fix_unexpected_token_in_expr_2731.rs`. This incrementally increases the CPAN clean rate toward the 90% Phase A target.

**The trajectory this opens**: Each fixed sub-pattern removes N files from the `unexpected_token_in_expr` bucket, improving the baseline. The ratchet mechanism means clean counts can only increase — this is debt paydown, not debt accumulation.

**The trajectory this closes**: Nothing is closed. Decomposing into follow-up issues keeps all options open — the issues can be prioritized, split further, or held based on builder bandwidth.

## Recommendations

### 1. Clarify the Corpus Scope Before Filing Issues
The 13 unique files split between:
- **3 accessible files** (English.pm, File/Copy.pm, IPC/Cmd.pm — Perl 5.38)
- **10 inaccessible files** (7 CPAN + 3 Perl 5.38.2 variants)

The 7 inaccessible CPAN files are the ones that matter most for the CORPUS_ROADMAP's stated goal of 90% CPAN coverage. Before filing issues, the builder should clarify whether:
- These files should be downloaded and added to the corpus for this work
- OR the issues should be filed based on the corpus sweep's error location data alone (without direct file inspection)

### 2. Distinguish Sub-Patterns That Are Already Partially Fixed
The verification agent found that `*\`` and `*'` (typeglob with backtick/tick suffix) are the ACTUAL failing patterns in English.pm — NOT `*+` as the research agent suggested. The existing `fix_unexpected_token_in_expr_2731.rs` Pattern C covers `*^N`, `*-{ARRAY}`, `*+{ARRAY}` but NOT `` *` `` or `*'`. This means the new sub-pattern is "typeglob suffix with `` ` `` or `'`" — filed as a distinct issue from the already-covered Pattern C.

### 3. Watch for Error Cascade Effects
The plan-reviewer correctly identified that fixing one sub-pattern in a file may reveal a different sub-pattern. English.pm likely has BOTH a fixed pattern (from #2731) AND new patterns. The issues should clearly state: "this is the FIRST error encountered when parsing, not necessarily the root cause of all subsequent errors in this file."

## Long-Term Impact

**Positive — This improves parser maintainability.** Each new sub-pattern gets its own test fixture in `fix_unexpected_token_in_expr_2731.rs`. Future developers can see at a glance what constructs are known-to-work and what the boundary of the parser's expression coverage is.

**Positive — This enables parallelization.** When the bucket is decomposed into 5 distinct issues with clear root causes, 5 builders can work simultaneously without stepping on each other's toes. This is essential for hitting the Phase A timeline (5 builders, 4-5 weeks).

**Neutral — The `insta` test dependency issue.** The test compilation failure blocks verification in THIS environment, but the test infrastructure itself is sound. This is an execution constraint, not a structural issue with the approach.

**Risk — Incomplete environment access.** If the 7 inaccessible CPAN files represent a significant portion of the 26-entry bucket's patterns, this work may only partially address the bucket. The issues filed may not cover all distinct sub-patterns present in the 13 unique files.

## Questions the Pipeline Should Answer

1. **Scope boundary**: Should this work include downloading and analyzing the 7 inaccessible CPAN files? Or is filing issues for only the 3 accessible files acceptable as "partial progress" toward the full bucket?

2. **Relationship to blockers.yaml**: The blockers.yaml marks `unexpected_token_in_expr` as "fixed" via #2731. Should this work update blockers.yaml to reflect that the bucket still has 26 entries in the system corpus, or is the system corpus considered stale relative to the CPAN corpus baseline?

3. **Test infrastructure**: The `insta` snapshot testing dependency is broken — is this being tracked as a separate issue? It blocks test-driven verification of any fix the builder produces.

4. **Issue ownership**: Who will pick up the follow-up issues this work produces? If no builder is assigned, the issues will sit open and the bucket won't actually decrease.

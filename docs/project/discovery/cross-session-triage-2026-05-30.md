# Cross-Session Discovery Triage Map — 2026-05-30

Read-only consolidation of **today's open candidate issues** (84 open, created 2026-05-30; ~33 carry the Issue-Discovery lane signature). Multiple discovery sessions ran concurrently, so this map exists to **stop the plan-review lane from doing duplicate work**. No issues were closed or deduped here — this is advisory; consolidation decisions belong to plan-review per the "don't dedupe unilaterally" rule.

> This session filed 10 of these (#956, #957, #958, #959, #960, #962, #985, #989, #992, #998). The rest are from concurrent sessions.

## Cross-session near-duplicates (recommend consolidation at plan-review)

| Issues | Overlap (verified) | Recommendation |
|--------|--------------------|----------------|
| **#973 ⊂ #998** | Both target `get_text_around_offset` (util/mod.rs:419-422) char-boundary panic. #998 additionally covers `completion.rs:714` and `:730-742`. | True component dup — co-review; fold #973 into #998's "C", or scope #998 to A/B and let #973 own C. **Two builders should not both fix util/mod.rs.** |
| **#945 ≈ #962** | Both cover `index.md` feature-count drift (116, non-existent "PR #4107") vs generated `lsp.md`. #962 also covers features.toml 125 vs [meta] 119. | Consolidate into one docs-drift issue (denominator + stale PR ref). |
| **#947 ≈ #961** | Both cover status-docs `v0.14.0` vs workspace `0.15.0` version drift (CLAUDE.md header vs generated docs). | Near-dup; merge. |
| **#954 ⟂ #992** | Same "silent-pass test" failure mode, **different tests**: #954 = `dap_integration_test.rs` (+2); #992 = LSP type_definition/implementation + parse-stress + streaming-completion. | Sibling — co-review as one test-hardening theme; do **not** close either (distinct surfaces). |
| **#984 ⟂ #956** | Same UTF-8 rename class, **different mechanism**: #984 = byte offset used as `Vec<char>` index (rename.rs:583/989); #956 = byte word-boundary check (workspace_rename.rs:489). | Sibling — co-review; a shared fix may resolve both. |
| **#969 ⟂ held UX-P4** | Same "error message names a non-existent/wrong setting" class: #969 = `perl-lsp.perl.path` (onboarding.ts:52); UX-P4 (this session, not filed) = `perltidyConfig` for a missing binary (formattingErrors.ts:42). | Fold UX-P4 into #969 (cross-ref comment added). |
| **#895 ≡ #898**, **#896 ≡ #899** | Identical titles (DAP lifecycle exec-control / ordering-matrix), differ only in `size/L` vs `size/M`. | Likely exact dup pairs from one session — plan-review should merge each pair. |

## Clusters (recommend co-review / sequencing — not merging)

- **UTF-8 byte/char-boundary** (highest impact — server crashes + edit corruption): **#750, #956, #973, #984, #998**. A single shared `clamp_to_char_boundary` / byte↔char helper + one hardening epic would likely close most of these. The recurring root: byte offsets from `pos16_to_offset` used directly for `&str` slicing or `Vec<char>` indexing.
- **DAP stackTrace correctness**: **#803** (threadId validation), **#933** (stale first frame on degraded transport), **#963** (totalFrames = paginated slice len), **#964** (stack_frames never cleared on resume), **#995** (fabricated placeholder frame). Strong candidates for one stackTrace-correctness pass.
- **DAP lifecycle / exec-control**: #895/#898, #896/#899, #901, #902.
- **docs-drift / version**: #945, #947, #960, #961, #962 (+ root cause: **#958** — the dead `post-merge-status` workflow means generated docs never refresh).
- **workspace-facts / module-resolution**: #811, #812, #813, #894, #941, #955, #970, #971, #983, #989 (inheritance/@ISA, multi-root leakage, block-form package, dynamic resolution). **#812** has a verified root-cause comment (PackageGraphIndex never wired).
- **settings / UX error-copy**: #917, #968, #969 (+ held UX-P4).
- **`gh` CLI unavailable in MCP/web env**: #946 (control plane hardcodes `gh` across 82 files), #972 (scout-dedup uses `gh`). Same substrate gap.
- **test-quality silent-pass**: #949, #954, #992.
- **nodekind lane**: #910–#915, #976, #993.

## Process lesson (multi-instance dedup)
My pre-file dedup search for #998 used a conjunctive query (`completion … analyze_context … get_text_around_offset`); GitHub AND-matches terms, so it missed #973 (which mentions none of "completion"/"analyze_context"). **Fix for multi-instance runs:** dedup on the single narrowest distinctive token (e.g. the bare function name), and run a central cross-session triage (this doc) on each cycle. Search-index lag (~minutes) also means same-cycle filings won't see each other — central triage is the backstop.

## What was done from this map (non-destructive)
- Cross-ref comments added (suggesting co-review, **not** closing): #973↔#998, #962↔#945, #969←UX-P4.
- PR #967 linked to its tracking issue **#942** ("Establish Issue Discovery lane").
- No issues closed, relabeled, or deduped — those are plan-review/maintainer decisions.

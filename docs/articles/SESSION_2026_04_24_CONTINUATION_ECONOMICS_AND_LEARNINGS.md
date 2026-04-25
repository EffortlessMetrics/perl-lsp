# Session Economics: 2026-04-24 Continuation — Security Cluster + Parser Accuracy Closeout

**Session window:** 2026-04-24 ~14:00 UTC → ~17:30 UTC (continuation after SESSION_2026_04_24_ECONOMICS.md)
**Companion memory files:** `memory/feedback_master_bit_rot_recurrence_pattern.md`, `memory/feedback_label_skill_silent_failure.md`, `memory/feedback_deep_review_bug_catch_roi.md`

## 1. Quota Burn Snapshot

Numbers taken at session close.

| Quota | Session used | Weekly used | Weekly remaining |
|---|---|---|---|
| Claude (Sonnet) | 35% | 43% | 57% |
| Codex | 11% | 66% | 34% |

The asymmetry is intentional. Codex is burning weekly quota at ~1.5x the rate of Claude. This matches the "cheap generation, premium verification" architecture: Codex floods the intake queue with candidates; Claude verifies, reviews, and merges the keepers. The Codex:Claude weekly burn ratio of ~1.5:1 is the budget expression of that architecture.

## 2. Session-2 Merges (since SESSION_2026_04_24_ECONOMICS.md)

| Metric | Count | Source |
|---|---|---|
| PRs merged (14:00–17:30 UTC) | **17** | `gh pr list --state merged --search "merged:>2026-04-24T14:00:00Z"` |
| PRs closed not merged (14:00–17:30 UTC) | **140** | `gh pr list --state closed --search "closed:>2026-04-24T14:00:00Z -is:merged"` |
| PRs resolved total (session 2) | **157** | merged + closed |
| Master commits pushed (session 2) | **17** | `git log --oneline origin/master --since="2026-04-24T14:00:00Z"` |
| Open PR queue at session close | **496** | `gh pr list --state open` |

For reference, session 1 (00:00–14:00 UTC) produced 91 merges and 219 closures. Session 2 ran shorter but produced a higher-quality merge batch (mostly directed parser-accuracy closes rather than triage-by-volume).

### Security cluster (8 PRs, admin-merged ~16:01–16:09 UTC)

Each fixes a distinct vulnerability in a different crate. See section 7 for the cross-layer reasoning.

| PR | Crate | Vulnerability fixed |
|---|---|---|
| **#6219** | `perl-module-resolution` | Absolute `use lib` roots bypass workspace boundary — path traversal / information disclosure |
| **#6220** | `perl-pragma` | Unbounded warning category accumulation — memory DoS |
| **#6221** | `perl-type-definition` | Cross-document re-parse bypasses large-file + binary safeguards — parse-based DoS |
| **#6222** | `perl-config` | `.perl-lsp.toml` read without size/type checks — workspace-local DoS via oversized/pipe file |
| **#6223** | `perl-refactoring` | Inline-all edits not scoped to definition tree — incorrect multi-file edit blast radius |
| **#6225** | `perl-type-hierarchy` | C3 MRO per-parent visited sets permit cycle → stack overflow |
| **#6226** | `perl-parser` | Quadratic heredoc location scans — algorithmic DoS on large source files |
| **#6227** | `perl-lsp-code-actions` | Quadratic line offset scans — algorithmic DoS |

### Parser accuracy (8 PRs)

| PR | Change |
|---|---|
| **#6282** | Preserve ternary and low-precedence binding after bare calls |
| **#6292** | Close remaining comma and list-builtin corpus failures |
| **#6296** | Classify and close delimiter recovery corpus buckets |
| **#6266** | Harden transliteration parsing and ratchet regressions |
| **#6287** | Recover incomplete interpolation indexing in strings |
| **#6018** | Harden batch edit normalization and UTF-8 boundary fallback |
| **#6308** | Add recovery-salvage metrics for accuracy closeout |
| **#6328** | Add benchmark suite for `perl-pragma` build/query workloads |

### Master bit-rot fixes

| PR | What it fixed | When |
|---|---|---|
| **#6286** | Format-string escape + four formatting violations blocking 30+ PRs | 14:19 UTC |
| **#6163** | Correct ci-scope package name after `perl-workspace` rename | 10:23 UTC (session 1 tail) |

### Docs

| PR | Content | When |
|---|---|---|
| **#6106** | Session learnings from 2026-04-24 throughput cycle | 08:23 UTC |
| **#6148** | Economic maturity + deep-review catalog + architecture audit | 08:57 UTC |
| **#6161** | Extended throughput session retrospective | 10:21 UTC |

## 3. Closures via Ensemble

The spec estimates ~93 Codex 4-shot duplicate closures this session phase (44 initial + 13 pragma + 36 role-predicate/symbol). These represent Codex cluster triage: for each design concern, one winner is selected and merged; the remaining 3–4 variants are closed with a cross-reference comment identifying why each was superseded.

The ~75% closure rate on incoming Codex PRs is now the observed baseline across multiple waves. It is not waste — it is the selection mechanism. The generation budget for a kept PR includes approximately 4 Codex task slots (1 winner + ~3 dupes) plus a full verification pipeline pass. Section 9 develops the economics.

## 4. Master Bit-Rot Recurrence Pattern

Four master bit-rot incidents occurred on 2026-04-24, each blocking 15–40 waiting PRs. The pattern is now formally documented in `memory/feedback_master_bit_rot_recurrence_pattern.md`.

| Incident | Blocked PRs (est.) | Investigation time | Fix-to-unblock time |
|---|---|---|---|
| `lifecycle.rs` compile + `parser_tests` fmt (#5749) | ~10 | ~15 min | ~10 min |
| `incremental_v2.rs` + `formatting.rs` escape (#5751, #5783) | ~15 | ~20 min | ~15 min |
| `cargo xtask fmt` cascade — 30 files, 2 visible in CI (#5965) | ~20 | ~30 min | ~25 min |
| Windows `canonicalize` short-name mismatch (#5986) | ~18 | ~40 min | ~20 min |
| Format-string escape + four formatting files (#6286) | ~30 | ~10 min | ~8 min |

**Pattern recognition value:** Once the pattern is identified — 3+ PRs failing an identical CI gate simultaneously — the correct action is: investigate master directly (not the individual PRs), author a narrow fix, admin-merge it past stale CI, then `gh pr update-branch` the blocked cluster. Average cost of pattern-match path: ~10 min. Average cost of miss-detect path (treating each PR as having its own bug): 2–3 hours of premium agent investigation time across the cluster.

**The xtask fmt cascade failure mode** recurred here: `cargo xtask fmt` aborts on first failure, so CI reports 2 files with violations while the actual blast radius is 20–30 files. The correct diagnostic is to run `cargo xtask fmt` locally (not `--check`) and observe total scope before concluding the fix is small.

Playbook: `memory/feedback_master_bit_rot_cascade_fixes.md`

## 5. New Learnings This Session

### Label-skill silent failure

Agents report "label X set" in their verdict comment, but labels frequently do not land. Observed ~80% silent-fail rate on one cluster during this session. The failure is silent at the agent level because the `gh label add` call may succeed with exit 0 but the label is not visible in subsequent `gh pr view --json labels` output if there is a rate limit or transient API error.

**Implication:** When ops reports "no merge-ready PRs" after a wave of sign-offs, check label drift before concluding the queue is dry. Bulk-apply missing labels from the orchestrator level via direct `gh pr edit --add-label` rather than relying on agent-level label writes.

Reference: `memory/feedback_label_skill_silent_failure.md`

### Local `origin/master` shadow branch blocks worktrees

A local branch literally named `origin/master` (created, for example, by a checkout command that went wrong) causes `git worktree add` to fail with "ambiguous object name" across the entire worktree set, blocking a whole agent wave identically. The fix is `git branch -D origin/master` in the main checkout.

Diagnostic: when an entire wave of worktree spawns fails with the same error, check `git branch -a | grep master` before investigating individual worktrees.

Reference: `memory/feedback_ambiguous_origin_master_branch.md`

### Ensemble posture: 0 / 1 / N, with N most common

Over multiple Codex burst waves, three postures have been observed:

- **0 keepers**: All variants miss the target (hallucination, scope drift, wrong crate). Close all with documentation. No merge.
- **1 keeper**: One variant is clearly correct. Merge it. Close the rest with the winner's cross-reference.
- **N keepers** (N typically 2–4): Each variant fixes a distinct vulnerability in a distinct crate. All are correct and non-redundant. Keep all, merge in a batch.

The security cluster (#6219–#6227) is the canonical N-keepers case. Eight PRs, eight crates, eight distinct vulnerabilities — cross-layer is not redundant. Treating it as a 1-keeper problem would have discarded seven real fixes.

Reference: `memory/feedback_multigate_catches_cheap_model_drift.md`

### Upstream research reduces verification burden

Pre-planning Codex task batches with ChatGPT Pro (reading recent Claude session history + current maintainer direction) reduces hallucination rates and scope drift in the resulting PRs. The investment is ~3–5 minutes per batch prompt and is performed with commodity web threads, not premium compute. The savings are downstream: fewer "not a real fix" closures, tighter PR scope, fewer haiku-review cycles.

This session's parser-accuracy wave (8 PRs, all merged) was preceded by such a pre-planning step. The security cluster was also pre-planned with explicit cross-crate scope constraints.

Reference: `memory/feedback_upstream_research_improves_pr_quality.md`

### Prompt generation is cheap web thread work

The upstream research + prompt generation loop is two commodity web threads: one reading recent Claude session artifacts (PR comments, memory files), one reading maintainer direction (ROADMAP.md, CLAUDE.md, recent merged PR titles). It requires no premium Codex or Claude budget. The full-stack economics stay favorable because the premium budget is reserved for verification, review, and merge — the stages where correctness matters.

Reference: `memory/feedback_prompt_generation_is_cheap_web_thread.md`

## 6. Deep-Review Catch Catalog (Session 2)

The following correctness bugs were caught by the Sonnet deep-review pass. Each had passed Haiku standards review without finding.

| PR | Bug class | What was caught |
|---|---|---|
| **#6052** | Security / correctness | 3 Windows `cmd.exe` bugs: unescaped metacharacters in `.bat`/`.cmd` wrapper invocation, missing `/D /S /C` quoting, `%` not doubled — command injection vector |
| **#6057** | Test validity | `tolerance_pct=0.0` in UX scorecard baseline created a zero-tolerance check that would fail on any floating-point noise; baseline was vacuous |
| **#6157** | Safety / panic | UTF-8 boundary: `rfind`-based byte offset in `detect_hash_key_context` lands inside multi-byte codepoint; slice-time panic on non-ASCII input (DoS) |
| **#6221** | Pre-existing bug | `foreach` double-visit in cross-document type scan: re-parse loop visited documents twice per lookup; correctness and performance bug pre-existing the PR, caught as bonus |
| **#6155** | Test validity | Vacuous symlink-escape test: `is_symlink() && !is_dir()` guard was always true for the test fixture (no dead symlinks in test data); test never constrained the guard |
| **#6159** | Spec misinterpretation | `willRenameFiles` framed as a security boundary; LSP spec defines it as a pre-rename notification for workspace-internal coordination, not a trust boundary. The security framing was the bug, not the implementation |
| **#6219** | Security | Embedded `..` components not stripped before `strip_prefix` check — path traversal bypass even after workspace root validation |
| C3 conflict | Design | Diamond linearization tiebreak: two deep-review passes produced conflicting recommendations on cycle-detection strategy; resolved by reading the C3 spec directly and using shared `visited` set (PR #6225) |
| **#6204** | Correctness | Missing `USERPROFILE` and `PREFIX` cache keys in environment hash — cache miss on every lookup for standard Perl tool paths |
| **#6328** | Correctness | Dead-code benchmark: `score` variable accumulated but was never read or returned; benchmark measured allocation cost only, not the claimed query cost |
| **#6227** | Off-by-one | `lines().count()` over-counts by 1 on CRLF line endings — `\r\n` produces an extra empty string at end on some Rust iterator implementations; affects line offset computation on Windows-formatted files |

Total distinct deep-review findings in session 2: **11**.

Consistent with the Era 7 pattern: every PR reviewed had at least one finding. The haiku standards pass caught none of these. See `docs/articles/DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md` for the full catalog including session 1.

## 7. Ensemble Triage Patterns at Scale

After multiple 4-shot Codex burst waves, ~75% closure rate is the observed baseline. For every 100 incoming Codex PRs, approximately 25 are merged and 75 are closed as duplicates or misses.

The exception to 1-winner triage is the **N-keepers cross-layer pattern**. The security cluster is the clearest example:

- 8 PRs, 8 crates (`perl-module-resolution`, `perl-pragma`, `perl-type-definition`, `perl-config`, `perl-refactoring`, `perl-type-hierarchy`, `perl-parser`, `perl-lsp-code-actions`)
- Each fixes a distinct vulnerability class (path traversal, memory DoS, parse DoS, config DoS, blast radius, stack overflow, algorithmic DoS x2)
- No two PRs touch the same file

This is not a cluster of redundant fixes. "Cross-layer" means each PR is the sole fix for its vulnerability in its crate. The correct posture is N-keepers, not 1-keeper.

**Rule of thumb:** When a Codex burst produces PRs that each touch different files and crates, default to N-keepers. When they overlap on the same file or function, triage for the best variant and close the rest.

The 75% closure rate is not a problem to be fixed. It is the natural selection ratio given that Codex generates 4 variants per design concern and the best-fit rate per variant is ~25%. Attempts to raise the keep rate by loosening the quality bar produce more noise, not more signal.

## 8. Throughput Bottleneck Identification

With ensemble triage efficient and master bit-rot pattern-recognized, the structural ceiling has moved.

**Previous bottlenecks (addressed):**
- Master bit-rot cascades: now pattern-recognized, ~10 min fix-to-unblock
- Duplicate triage at scale: now handled by ensemble-curator with documented postures
- Label drift: now observable via direct `gh pr view --json labels` verification

**Current structural ceiling: Windows CI latency**

Windows sandbox test runs take 45–90 minutes per PR. The UX regression gate (`perl-lsp-ux-tests`) has a persistent flake component that adds retry overhead. At current merge throughput, Windows CI is the rate-limiting step: a batch of 3 PRs with Windows test coverage takes 2–4 hours of wall-clock time to clear.

**Forward work:**

1. **UX Regression Gate persistent flake**: The `resolve_binary()` path in `crates/perl-lsp-ux-tests/src/lib.rs` uses `option_env!("CARGO_BIN_EXE_perl-lsp")` which on Windows strips backslashes, producing `H:CodeRustperl-lsp` instead of `H:\Code\...`. This may be the underlying cause of the periodic UX gate timeout failures (not the 10s timeout bumped in #5097).

2. **Windows path normalization audit**: `std::fs::canonicalize` short-name expansion (#5986) pattern may recur in other crates. An audit pass on all uses of `canonicalize` against caller-supplied path comparison would preempt future Windows-only CI failures.

3. **Slow Windows guardrails**: The Windows Guardrails lane serializes all path-validation changes. At current intake rate, this creates a queue starvation risk for security PRs that touch path-handling code.

## 9. Quota Economics: Forward-Looking

At current cadence:

- Codex burns ~1.5× Claude weekly quota
- ~75% of incoming Codex PRs are closed (not merged)
- The retained ~25% require a full verification pipeline: haiku standards review + Sonnet deep review + green-TDD + green-refactor + CI

**Cost per retained keeper (rough model):**

| Component | Cost tier | Count |
|---|---|---|
| Codex generation (1 winner + ~3 dupes) | Generation | ~4 Codex task slots |
| Haiku standards review | Verification | 1 haiku pass |
| Sonnet deep review | Verification | 1 sonnet pass |
| Green-TDD hardening | Verification | 1 haiku pass |
| Green-refactor | Verification | 1 sonnet pass |
| CI wall-clock | Infrastructure | 45–90 min (Windows) |

Total: ~2× generation-tier cost + ~1× verification-tier cost per keeper. The verification tier dominates because sonnet (deep review, green-refactor) is ~10× the cost of generation-tier Codex tasks.

**Sustainability assessment:** The model is sustainable at current throughput. The 1.5:1 Codex:Claude weekly burn ratio means Codex hits its weekly quota ~2–3 days before Claude does, creating a natural pacing mechanism. The bottleneck is Windows CI latency, not quota burn rate. Increasing Codex burst size without increasing Windows CI parallelism would saturate the merge queue, not accelerate it.

**The "cheap generation + premium verification" architecture holds.** The risk is not Codex saturation — it is verification-tier saturation if deep-review findings per PR increases. The Era 7 finding rate (100% of PRs with at least one finding) suggests the current input quality from Codex is not yet at the point where deep review becomes marginal.

---

_Related: `docs/articles/SESSION_2026_04_24_ECONOMICS.md`, `docs/articles/DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md`, `docs/articles/VERIFICATION_LADDER_PER_LAYER_ROI.md`_

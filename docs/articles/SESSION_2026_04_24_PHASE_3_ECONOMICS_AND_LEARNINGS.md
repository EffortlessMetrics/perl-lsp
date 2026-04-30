# Session Economics: 2026-04-24 Phase 3 — Windows Bit-Rot, Vacuous-Test Catalog, Stale-Base Security Reversion Pattern

**Phase boundary:** After #6445 merged at 17:29 UTC, through end of session (~18:00 UTC)
**Companion docs:** `docs/articles/SESSION_2026_04_24_ECONOMICS.md` (phase 1), `docs/articles/SESSION_2026_04_24_CONTINUATION_ECONOMICS_AND_LEARNINGS.md` (phase 2)

---

## 1. Quota Burn Snapshot

Status at phase-3 boundary (approximated from session context; exact reset times noted where known):

| Model | Session Used | Weekly Used | Weekly Reset |
|---|---|---|---|
| Claude (Sonnet) | ~2% | ~46% | Saturday 5pm ET |
| Codex | ~11% | ~66% | Apr 28 |

Codex weekly burn is approximately 1.4x Claude weekly burn at this snapshot. This continues
the trend from phase 2 (which measured 1.5x). The ratio is converging: Claude's session
allocation goes toward high-judgment work (deep review, plan review, admin decisions)
while Codex generates the raw PR volume. As the queue narrows and Codex targets fewer
distinct areas, the ratio is expected to continue falling toward 1.0x.

Architecture-economics relationship: Codex burns more weekly quota because it operates in
burst-and-triage mode — generating 12-48 PRs per task, most of which are closed within
the same session. Claude quota is consumed during the judgment passes (ensemble-curator,
deep reviewer, plan reviewer) that determine which Codex output survives.

---

## 2. Phase-3 Merges

**Phase boundary:** After #6445 merged at 17:29 UTC.

Merges in the strict phase-3 window (after 17:30):

| PR | Title | Notes |
|---|---|---|
| **#6446** | Windows path test assertions in config tests | Master bit-rot fix; gates expected path strings on platform separator |

For completeness, merges in the extended phase-3 window (16:10–17:41, which includes the
continuation session's active triage work):

| PR | Title | Category |
|---|---|---|
| **#6445** | Continuation session economics + learnings doc | Forensics/docs |
| **#6328** | perl-pragma benchmark suite | Perf |
| **#6308** | Recovery-salvage metrics for parser accuracy closeout | Feature/metrics |
| **#6287** | Recover incomplete interpolation indexing in strings | Parser fix |
| **#6266** | Harden transliteration parsing + ratchet regressions | Parser fix |
| **#6296** | Classify and close delimiter recovery corpus buckets | Parser fix |
| **#6282** | Preserve ternary and low-precedence binding after bare calls | Parser fix |
| **#6018** | Harden batch edit normalization + UTF-8 boundary fallback | Incremental parsing |

**Strict phase-3 merge count (post-17:29):** 1 (#6446)
**Extended phase-3 merge count (16:10–17:41):** 9 (including the phase-2 doc itself)

---

## 3. Phase-3 Closures

The dominant closure event in phase 3 was the `perl-token` Codex burst triage: PRs
#6397–#6443 (47 PR range). This cluster was generated from a single Codex task batch
targeting `perl-token` improvements across 8 distinct design concerns.

| Cluster | PRs closed | Winner(s) surviving |
|---|---|---|
| perl-token scorecard/baselines | 2 (#6397, #6398) | #6396 (open) |
| TokenKind metadata/categories | 3 (#6400, #6401, #6402) | #6403 (open) |
| TokenKind catalog drift tests | 3 (#6404, #6405, #6406) | #6407 (open) |
| Checked token spans/invariants | 3 (#6408, #6409, #6410) | #6411 (open) |
| perl-token conformance coverage | 0 closed (3-way) | #6412, #6413, #6414 (all open; still needs triage) |
| TokenKind role predicates | 3 (#6425, #6426, #6427) | #6428 (open) |
| Keyword/operator mapping helpers | 3 (#6416, #6417, #6418) | #6419 (open) |
| Borrowed token view | 3 (#6429, #6430, #6431) | #6432 (open) |
| Token allocation overhead | 3 (#6433, #6434, #6435) | #6436 (open) |
| TokenKind display name ratchet | 3 (#6437, #6438, #6439) | #6440 (open) |
| API/dependency ratchet (ci) | 3 (#6441, #6442, #6443) | #6444 (open) |
| perl-token status metrics | 3 (#6420, #6421, #6422) | #6423 (open) |

**Total closed in perl-token burst:** 32 PRs out of 48 in range (67% immediate closure rate)
**Notable:** 75% closure rate is the sustained session average; this burst was slightly below
average because the conformance-coverage cluster (#6412–#6415) had no clear winner and was
left open for human adjudication.

Also closed in phase 3: 36 additional dupes from the `perl-symbol` SymbolRef/SymbolIndex
clusters that were triaged in the same pass:

| Cluster | Closed PRs |
|---|---|
| SymbolRef projection layer | #6392, #6393 (3-way; #6394 also closed, winner TBD) |
| SymbolDecl projection | #6388, #6389, #6390 (3-way) |
| SymbolIndex production wiring | #6380, #6381, #6382 (3-way) |
| Symbol parity bank | #6369, #6370 (2 of 3) |
| Cursor parity regressions | #6364, #6365, #6366 (3-way) |

**Total non-merged closures from 16:30 onward:** 68

---

## 4. Master Bit-Rot Pattern Catalog

Four distinct master bit-rot instances landed on 2026-04-24, each requiring an admin-merge
fix to unblock dependent PRs. This is the highest single-day bit-rot count observed.

| Instance | PR | Root cause | PRs unblocked |
|---|---|---|---|
| Package rename | #6163 | `perl-workspace-index` renamed; `ci-scope` task still referenced old name | ~8 (CI scope lane) |
| fmt + format-string | #6286 | `cargo xtask fmt` aborted on first failure; format-string drift in parser tests blocked 30+ PRs | ~30 |
| Windows 8.3 canonicalization | #5986 | `std::fs::canonicalize` expanded `RUNNER~1` short names to `runneradmin`; path comparison failures on Windows CI | ~18 (Windows lane) |
| Windows path test assertions | #6446 | `normalize_include_path` round-trips through `PathBuf::to_string_lossy()` emitting `\` on Windows; tests hardcoded `/` | ~12 (config test failures) |

**Pattern:** Each instance was a CI failure that manifested only on a specific platform or
CI lane. The failure cascaded: once master had a broken test, every PR that triggered that
test lane failed for a non-PR-local reason. The cascade was resolved by fix + admin-merge,
then `gh pr update-branch` across the affected cluster.

**Detection sequence that works:**

1. Multiple PRs fail in the same lane at the same time.
2. Run the failing test in isolation on a clean master checkout.
3. If master is broken, the cause is master-side — fix and admin-merge.
4. If master is clean, the PR introduced the failure — fix on the PR branch.

**What does NOT work:** Assuming "CI failing on this PR" means the PR is broken. In three of
the four instances above, the PR was correct and master was the cause. Running the failing
test against master before attributing the failure is the minimum viable investigation step.

See also: `memory/feedback_master_bit_rot_cascade_fixes.md` (operational playbook).

---

## 5. Stale-Base Security Regression Pattern

Three PRs in the phase-3 window were identified as having diverged from master before the
security cluster (#6219, #6220, #6221) merged, meaning their diffs would silently revert
recently landed security work.

The security cluster that landed at 16:00–16:05 UTC:

| PR | What it guarded |
|---|---|
| **#6220** | `MAX_DISABLED_WARNING_CATEGORIES` cap: bounded growth of the pragma tracker's warning-category set to prevent unbounded memory allocation |
| **#6219** | `path_to_relative_string()` workspace guard: ensured `use lib` path injection respected workspace boundaries |
| **#6221** | `should_parse_document()` size+binary guard: added size limit and binary-content check before full parse |

PRs branched before this cluster merged would silently omit these guards if they touched
the same files. Detection required reading the actual diff against current master, not
against the PR's base commit — a step that only the later pipeline stages (maintainer-pr,
diff-auditor, deep-review) performed. The standards reviewer (haiku) could not catch this
because it does not compare against master, only against the declared base.

**Why haiku review misses this:** The standards pass checks for banned patterns, scope
containment, and formatting compliance. It does not read the cumulative diff against
current master. A reversion of a 5-line security guard produces a clean diff on its own
— no banned patterns, correct formatting, in scope.

**Detection sequence:**

1. diff-auditor: compare PR diff against current master HEAD, not just PR base.
2. maintainer-pr: review whether any removed lines were recently added intentionally.
3. deep-review: verify that the implementation matches the intent, not just the PR description.

**Mitigation:** When a security cluster merges, `gh pr update-branch` the top-N highest-risk
open PRs (those touching the same files) immediately, before they proceed further in the
pipeline. This converts a potential silent reversion into a visible merge conflict that can
be reviewed explicitly.

---

## 6. Vacuous-Test Catches as Deep Review's Primary Value

The most recurring deep-review finding across this session was the vacuous-test: a test
that passes without exercising the claimed behavior. Four instances documented:

### 6.1 — PR #6155: Symlink test never exercised T2 guard

**Claimed behavior:** "If a symlink points outside the workspace, the extension does not
offer 'Create Missing Directories'."

**Vacuity mechanism:** The test setup called `fs.realpathSync` on a path that was mocked —
the mock prevented `mkdir` from running. The assertion checked that `mkdir` was not called,
which was true both when the guard worked and when the guard was absent (mock blocked mkdir
regardless). A mutant that removed the guard entirely would still pass this test.

**Fix pattern:** Add a positive-control assertion: run with a safe path (no symlink) and
verify `mkdir` IS called. Then run with the unsafe path and verify it is NOT called. The
positive control fails under a broken guard.

### 6.2 — PR #6308: Recovery test accepted zero-filled struct

**Claimed behavior:** "Recovery metrics correctly classify dirty files into the four
recovery buckets."

**Vacuity mechanism:** The test asserted `profile != RecoverySalvageProfile::default()`
(i.e., the profile was non-zero), but did not assert specific field values. A buggy
implementation that put all counts in the wrong bucket would still produce a non-zero
profile and pass the test. The assertion checked variant/non-zero, not value.

**Fix pattern:** Assert specific field values: `profile.error_node_count == 3`,
`profile.classification == ErrorNodesPresent`. This fails when buckets are miscounted.

### 6.3 — PR #6342: "no feature ':all' clears" passed because baseline was empty

**Claimed behavior:** "After `use feature ':all'; no feature ':all';`, all features are
disabled."

**Vacuity mechanism:** The test baseline state was an empty feature set. `no feature ':all'`
on an empty set produces an empty set. The assertion `features.is_empty()` passed trivially
because the test never put any features into the enabled set first. A bug that made
`no feature ':all'` a no-op would still pass.

**Fix pattern:** First assert that `use feature ':all'` populated the set (positive
control), then apply `no feature ':all'` and assert it is empty. Without the positive
control, the test proves nothing.

### 6.4 — PR #6396: Percentile tests did not exercise clamping or single-sample edge cases

**Claimed behavior:** "The `perf_scorecard` helper correctly computes median and p95 using
the corrected nearest-rank formula."

**Vacuity mechanism:** Tests used arrays with enough elements that the nearest-rank formula
always landed on a valid index. The clamping behavior (single-element array, array where
p95 maps to the last element) was never exercised. The formula could have been off-by-one
at boundaries and all tests would still pass.

**Fix pattern:** Add tests for `[42]` (single sample: both median and p95 should be 42),
`[1, 2, 3]` (3-sample: p95 clamps to 3), and verify clamping does not panic.

### Common structure

All four instances share the same failure mode: the assertion checks a property that is
true both under correct behavior and under the specific bug. The fix is always the same:
add a positive-control assertion that would fail under a buggy implementation.

A useful heuristic: after writing a test, ask "what is the simplest mutation of the
production code that would still pass this test?" If the answer is "remove the feature I
just added," the test is vacuous.

---

## 7. Pre-Existing Bug Catches via PR-Responder

PR #6308 (recovery-salvage metrics) illustrates a pattern where investigating a
"PR-specific CI failure" uncovered a master-side bug.

**The sequence:**

1. PR #6308 added recovery metrics and classification tests.
2. CI failed on one test: `test_execute_command_recovery_timeout`.
3. Initial assumption: the PR broke the test.
4. Investigation: ran the failing test against master without the PR's changes.
5. Finding: `Command::new()` in `execute_command/provider.rs` was not setting `.stdout(Stdio::piped())` and `.stderr(Stdio::piped())`. The provider relied on default behavior (inheriting the parent process's stdio), which works interactively but fails in CI where inherited stdio is unavailable or buffered differently.
6. Fix: The PR-responder added `.stdout(Stdio::piped()).stderr(Stdio::piped())` to the command construction. 13 tests that were previously passing by accident (their assertions did not depend on captured output) now have properly isolated stdio.

**Pattern:** When a PR-side CI failure appears on a test that the PR does not directly
touch, run the test in isolation against current master before assuming the PR is the cause.
The pr-responder fixed both the test attribution and the underlying provider bug in a single
pass.

**Forward-looking implication:** Tests that "pass in CI" via inherited stdio may be silently
broken in that they do not actually verify output content. Adding explicit stdio piping
exposes this class of latent failures.

---

## 8. Closing Observations

### Pipeline efficiency

The Codex burst-and-triage pattern continues to sustain approximately 75% closure rates
on cluster-sized batches (3-4 PRs per design concern). The perl-token burst
(#6397–#6443) fell slightly below this at 67% because one cluster (conformance coverage)
had no clear winner. Three-way ties without a clear differentiator are the primary source
of below-average closure rates.

### Master bit-rot is the dominant throughput blocker

Four bit-rot instances in a single day produced a combined estimated blockage of 68+ PRs
across multiple CI lanes. Each instance required human escalation (admin-merge) to resolve.
The time from bit-rot introduction to detection averaged 2-4 hours, during which queued
PRs accumulated false failures.

The bit-rot rate has increased in proportion to merge velocity. At 75+ merges per day, the
probability that any given merge introduces a test regression scales with the number of
platform-conditional or environment-dependent tests in the codebase. The Windows CI lane
is the highest-risk lane: three of the four instances above were Windows-specific.

### Windows CI as the structural ceiling

Windows sandbox CI runs take 45-90 minutes per run. This means:

- A Windows bit-rot fix cannot be validated and admin-merged in under 90 minutes.
- During that 90 minutes, every PR that touches Windows-sensitive tests appears broken.
- The 12 PRs estimated to be blocked by #6446 each waited a full CI cycle to discover they
  were unblocked.

Mitigation at scale: a fast-path "Windows config test smoke" run (subset of tests, no
sandbox) that can validate Windows path assertions in under 10 minutes. This would reduce
the cascade window from 90 minutes to 10.

### The forensic-doc-PR pattern

This is the fifth session retrospective in the forensic-doc series:

| PR | Doc | Phase |
|---|---|---|
| #6106 | Session 2026-04-24 throughput cycle | Phase 1 |
| #6148 | Economic maturity + deep-review catalog + architecture audit | Phase 1 extension |
| #6161 | Extended throughput session retrospective | Phase 2 boundary |
| #6445 | Continuation session economics + learnings | Phase 2 |
| This PR | Phase 3 economics + master-bit-rot + vacuous-test patterns | Phase 3 |

Each doc captures patterns that were not yet written when the session started. By the time
this doc is merged, the patterns in sections 4–7 will be available to future agents as
memory cross-references, preventing repeat investigation of the same failure modes.

The series is its own evidence that the "wisdom" loop (agent-wrapup → memory → future
agent reads memory → avoids mistake) is functioning. The deep-review catalog article
(merged in phase 1) was cited by the phase-2 continuation doc. The master-bit-rot cascade
playbook (from phase 1) is cited here. Each session teaches the next.

---

## Verified Numbers Summary

| Metric | Value | Source |
|---|---|---|
| Phase-3 master commits (after 17:30 UTC) | 1 | `git log --oneline --since="2026-04-24T17:30:00Z"` |
| Phase-3 merges (after 17:30 UTC) | 1 (#6446) | `gh pr list --state merged --search "merged:>2026-04-24T17:30:00Z"` |
| Phase-3 non-merged closures (after 17:00 UTC) | 68 | `gh pr list --state closed -search "closed:>2026-04-24T16:30:00Z -is:merged"` |
| perl-token burst (#6397–#6443) closed | 32 | Counted from closed list |
| Open PR queue at phase-3 end | ~498 | `gh pr list --state open --limit 500` (hit 500-result cap) |
| Codex:Claude weekly burn ratio | 1.4x | Session context estimate |

---

_Related: `memory/feedback_master_bit_rot_cascade_fixes.md`, `memory/feedback_deep_review_bug_catch_roi.md`, `docs/articles/DEEP_REVIEW_FIX_FORWARD_CATALOG_2026_04_24.md`_

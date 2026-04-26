# 2026-04-25 — PR Queue Drain Session Forensics

**Session window**: ~5 hours, 2026-04-25
**Repo**: EffortlessMetrics/perl-lsp
**Operating mode**: Continuous swarm with ensemble triage focus, fix-forward reviewer-deep, batched ops merges
**Headline**: Drained ~108 duplicate-cluster PRs while preserving signal; merged 10; landed 14+ fix-forwards; cleared 66+ stale issues. Ended rate-limited on the GitHub API.

---

## Session metrics

| Metric | Value |
|---|---|
| **Open PRs at start** | ~410 |
| **Open PRs at end** | 327 (via `Link: rel="last"` header on `repos/EffortlessMetrics/perl-lsp/pulls?state=open&per_page=1`) |
| **Net delta** | −83 open PRs |
| **Master at start** | `7943a13e` |
| **Master at end** | `e6d5ed0a2` — `test(perl-lsp-rs): add workspace symbol snapshots (#0000) (#5426)` |
| **Session duration** | ~5 hours |
| **Dispatch waves** | 4 large dispatch waves + several follow-up sweeps |
| **Rate-limit terminus** | GraphQL API exhausted before final tally; REST `Link` header still served |

---

## Outcomes (estimated, agent-reported)

### Merges (10)
Drained the merge-ready tail end-to-end: scope-checked, deep-reviewed, ci-greened, diff-audited, ops-merged in batches of 3 with master-cascade between batches. Master advanced from `7943a13e` to `e6d5ed0a2` over the session.

### Cluster closures via duplicate-ensemble triage (~108)
Codex bursts from prior sessions had stacked dense duplicate clusters across several subsystems. Curated each cluster: pick winner by file-path coherence + edge-case coverage, extract test cases from losers, close losers with cross-ref to winner.

| Cluster | PRs closed |
|---|---|
| Lexer | 22 |
| Parser | 16 |
| Tree-sitter | 7 |
| Incremental | 9 |
| Workspace / symbol | 5 |
| Refactor #3522 | 9 |
| Parser closeout | 2 |
| Misc smaller | 12 |
| Refactor tooling cherry-closures | 2 |
| Hermes false-positive correction | 1 (reverted closure) |
| **Total** | **~108** |

### Cherry-pick + reopen (2)
- `#6717` — cherry-picked onto fresh master, reopened
- `#6718` — same

These two needed to survive a master root-rebuild that invalidated prior history.

### Fix-forward pushes (14+)
Reviewer-deep + diff-auditor pushed mechanical fixes directly to PR branches rather than bouncing back to builders. Saved an estimated 14+ builder round-trips. Pattern matches the `feedback_reviewer_deep_proactive_fixes` memory entry.

### Stale duplicate ISSUES closed (66+)
Companion sweep on the issue tracker after PR cluster curation. Each closed issue cross-references the surviving PR or canonical issue.

### Tracking issue filed (1)
- `#6715` — captures unresolved branch-contamination cluster + Unicode plan-review questions for next session

---

## Key learnings (link to memory entries)

### 1. xtask fmt cascade vs master bit-rot disambiguation
When N PRs fail the same gate identically, it's a master signal — but `xtask fmt` aborts on first file failure, so the CI report shows 2 files when the actual fix touches 30. Pattern documented in `feedback_master_bit_rot_cascade_fixes` and `feedback_master_bitrot_cascade_8plus_pattern`. Today's session reinforced: **always run `cargo xtask fmt` locally to enumerate the full set, don't trust the CI failure list**.

### 2. Master root-rebuild forces cherry-pick
When master is rebuilt (e.g. squash + history rewrite for a foundational PR), branches based on the old master need cherry-pick + reopen, not rebase. `#6717` and `#6718` followed this pattern. Rebase would have surfaced as "no commits to apply" because git saw the squashed commit as already present.

### 3. CRLF breaks xargs on Windows
Discovered while batch-closing PRs: `gh pr list --json number -q '.[].number' | xargs -I{} gh pr close {}` failed because PowerShell-piped `gh` output carried CRLF, and xargs passed the `\r` into the URL. Fix: pipe through `tr -d '\r'` before xargs, or use `gh` with `--jq` and a bash `for` loop instead.

### 4. CI workflow trigger observability gap
Several PRs sat in apparent merge-ready state with no CI runs visible — workflows had silently not triggered (likely after a force-push during rebase, GitHub deduplicated). Pattern: `gh pr checks <num>` shows "No checks reported" instead of "pending" or "failed". Mitigation: explicit `gh workflow run` to retrigger, or push an empty commit.

### 5. 20-min sandbox-fail-closed timeout too tight
A reviewer-deep agent ran `cargo test --workspace` and tripped the 20-min sandbox timeout in the middle of `perl-parser` test compilation. Test was correct, just slow. Either widen the timeout for full-workspace checks or scope to `--lib` per `feedback_ci_runs_lib_tests_only`.

### 6. Triage at scale validates ensemble economics
108 cluster closures in one session at ~30 sec of curator-agent time per cluster (vs. 5+ min of full review per PR) = **~10× throughput on duplicate burden**. The Codex ensemble pattern (`feedback_codex_ensemble_pattern`) is net-positive only with a curator step; without curation the queue grows monotonically.

### 7. Hermes attribution policy correction
Initially closed a hermes-flagged PR as cross-PR contamination, then reopened on re-read: the audit trail entries were *this PR's own* attribution records, not bleed from another PR. Policy clarification: **a PR may legitimately add audit-trail entries that name itself**. The contamination signal is when the audit trail names a *different* PR's work. Documented in the `check-agent-audit-trail` skill behavior.

### 8. Branch contamination from merge-master-not-rebase
A cluster of PRs (#6379, #6252, #6192, #6138, #6051, #5320, #5220, #5213) show contamination patterns consistent with `git merge master` instead of `git rebase master` — the diffs include unrelated merged work. Defer rebase-cleanup to next session; flagged in unresolved items below.

---

## Unresolved items for next session

### Branch contamination cluster (rebase needed)
PRs: `#6379`, `#6252`, `#6192`, `#6138`, `#6051`, `#5320`, `#5220`, `#5213`
Action: small targeted agent to verify contamination, then per-PR rebase-or-recreate.

### Hard-conflict cherry-picks (manual)
PRs: `#5711`, `#5710`, `#5695`
These need hand-rebase against current master; auto-rebase produced unresolvable conflicts.

### C21 Unicode plan-review
Decide: extend `#6098` vs. open new PR for emoji tag range. Plan-reviewer call next session.

### `#6001` PR Smoke FAILURE
Investigate why PR Smoke is failing — could be flaky, could be real. No diagnosis attempted today.

### Architecture choice for exporter metadata (#3416)
Two candidate PRs: `#5793` vs. `#5795`. Need maintainer-pr decision on which architecture wins.

### Maintainer-pr-reviewed labels pending (rate-limit recovery)
Six PRs need the `maintainer-pr-reviewed` label applied as soon as API quota resets:
`#5320`, `#5220`, `#5213`, `#5051`, `#5034`, `#5032`

(See `feedback_label_skill_silent_failure` — verify with `gh pr view --json labels` after applying.)

---

## Top 5 next-session priorities

1. **Verify branch contamination claim** — small targeted agent runs `git log master..HEAD` on each suspect PR and reports the unrelated commits. Cheap, decides scope of rebase work.
2. **Apply pending labels** — single-shot label catchup once API quota resets (`gh pr edit <num> --add-label maintainer-pr-reviewed` for the 6 PRs above).
3. **Drain remaining merge-ready** — ops sweep against `gh pr list --search "label:merge-ready"` after labels are caught up.
4. **Architecture decision on `#5793` vs `#5795`** — maintainer-pr-style synthesis comment, pick a winner, close the other.
5. **Master CI freshness check + cascade-update if needed** — `gh pr update-branch` against any PRs whose CI is now stale relative to current master `e6d5ed0a2`.

---

## Operational signals worth carrying forward

- **Cache TTL pacing held**: ran near-continuously, no >1.5h idle gap (`feedback_cache_ttl_session_pacing` respected).
- **Multi-gate caught real drift**: oppositional + research + diff-audit + deep-review surfaced 3 separate hallucinated APIs from Codex bursts. Reinforces `feedback_multigate_catches_cheap_model_drift`.
- **No agent killed mid-task**: `feedback_dont_kill_agents` respected; over-broad agents finished and contributed partial value.
- **Did not damage main `.git`**: no `rm -rf .git` paths attempted (`feedback_agent_damaged_main_checkout` respected).

---

*Doc written under rate-limit conditions; PR/issue counts above are agent-reported estimates — exact values can be reconciled next session by querying GitHub once quota resets.*

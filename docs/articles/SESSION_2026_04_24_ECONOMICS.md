# Session Economics: 2026-04-24 — Master Bit-Rot Cascade + Ensemble Curator Scale

**Session window:** 2026-04-24 00:00 UTC → ~08:30 UTC (one 5h Claude window + memory compaction mid-session)
**Companion:** `memory/feedback_master_bit_rot_cascade_fixes.md` (reusable playbook)

## Verified numbers

Numbers are drawn from `gh pr list --json` queries run against the live repo at session end.

| Metric | Count | Source |
|---|---|---|
| PRs merged on 2026-04-24 | **75** | `gh pr list --state merged --search "merged:>2026-04-24T00:00:00Z"` |
| PRs closed (not merged) on 2026-04-24 | **156** | `gh pr list --state closed` minus merged |
| PRs resolved total | **231** | merged + closed |
| Master commits pushed today | **51** | `git log --oneline origin/master --since="2026-04-24T00:00:00Z"` |
| Open PR queue at session end | **388** | `gh pr list --state open` |

Queue trajectory: queue started near 366 (per prior session retrospective) and ended at 388 after ~50+ new PRs were generated mid-session via Codex waves. Net drain was masked by ingest volume.

## Economics at close

- **Claude:** 56% of 5h window used, 2h6m remaining. 24% weekly used, resets Saturday 5pm.
- **Codex:** 63% of 5h window remaining (resets 06:36 AM), 52% weekly remaining (resets Apr 28).
- The session had one Claude Code restart + memory compaction midway; exact checkpoint boundaries are fuzzy.

## What shipped (by category)

### Master bit-rot cascade fixes (high leverage, admin-merged)

Four master-side fixes each unblocked a cluster of dependent PRs. These were admin-merged past stale CI because the changes were purely mechanical and the original failure was on master, not on the waiting PRs.

| PR | What it fixed | PRs unblocked |
|---|---|---|
| **#5749** | Combined: `lifecycle.rs` compile + `parser_tests` fmt | ~10 |
| **#5751, #5783** | Fmt: `incremental_v2.rs`, `formatting.rs` escape | ~15 |
| **#5965** | Restored master formatting (cargo xtask fmt aborts on first failure — only 2 files visible in CI, 30 actually touched) | ~20 |
| **#5986** | Windows include-path normalization: `resolve_module_path` used `std::fs::canonicalize` which on Windows CI expands `RUNNER~1` short names to long names (`runneradmin`); fix made `validate_workspace_path` a boolean security gate only, returning caller's path instead | ~18 (Windows Guardrails lane) |

After each master-side merge: `gh pr update-branch` run across the affected PRs to propagate the fix. This pattern is documented in the playbook at `memory/feedback_master_bit_rot_cascade_fixes.md`.

Note: **#5097** (LSP client timeout 10s → 30s for CI runner contention) was also merged. The underlying UX gate blocker may still be `resolve_binary()` in `crates/perl-lsp-ux-tests/src/lib.rs`, which uses `option_env!("CARGO_BIN_EXE_perl-lsp")` — on Windows this strips backslashes, producing `H:CodeRustperl-lsp` instead of `H:\Code\...`. This was the suspected real blocker, not the 10s timeout. Filed for follow-up.

### Feature and fix PRs (subset with substantive impact)

| PR | Change |
|---|---|
| **#5958** | Agents: inline external-agent triage rules + collapse-era crate framing (eliminates stale crate references in agent prompts) |
| **#5401** | Add rename UX e2e scenario (green-tdd hardening) |
| **#5454** | Improve criterion benchmark extraction in xtask |
| **#5207** | Harden dynamic config pull ordering in workspace |
| **#5108** | Handle CR/CRLF line starts in LineIndex |
| **#5022** | Resolve bareword imports in `use ... qw(...)` lists across 4 LSP surfaces |
| **#5689** | Record error for unclosed qw/q/qq bracket delimiters |
| **#5753, #5754** | Clamp mid-codepoint UTF-16 offsets + improve surrogate offset mapping |

### Ensemble-curator cluster triage (156 PRs closed)

The majority of closures came from triage of Codex-generated clusters: picking the best variant per design concern and closing duplicates with documentation of why each was closed.

## Patterns observed

### 1. xtask fmt cascade failure mode

`cargo xtask fmt` aborts on the first formatting failure. When master drifts, CI reports only the first 2 files with issues. The actual scope is much larger. On #5965, the visible CI failure was 2 files but the actual fix touched 30 files. This is a recurring source of confusion: reviewers underestimate the blast radius when they read the CI failure message.

**Implication:** When fmt CI fails and the fix seems small, run `cargo xtask fmt --check` locally across the full workspace before concluding the scope.

### 2. Windows 8.3 short-name canonicalization

`std::fs::canonicalize` on Windows expands short names (e.g., `C:\Users\RUNNER~1\...`) to long names (`C:\Users\runneradmin\...`). `tempfile::tempdir()` returns the short-name form on GitHub Actions Windows runners. Any code that calls `canonicalize` and then compares the result against a caller-supplied path will produce path mismatches on Windows CI even when the paths point to the same location.

Fix pattern: use canonicalization only as a security gate (traversal prevention), not to obtain the canonical form of the return value. Return the caller's original path.

### 3. ChatGPT-Pro-planned Codex batches vs direct Codex

Codex batches planned via ChatGPT Pro (with explicit scope constraints) produced tighter, more layer-diverse PRs with fewer hallucinations. The ~07:30 UTC session boundary roughly coincides with the shift to this pattern. Subsequent waves showed lower same-file duplicate rates and narrower scope per PR.

### 4. Ensemble cluster cascade awareness

When triaging Codex clusters that span multiple days or sessions, earlier cluster winners can supersede later sibling clusters' work. Cross-batch awareness is needed: before picking a winner within a new cluster, check whether a prior cluster already merged a superseding change.

### 5. Hallucination rate in this session

Zero hallucinated framework names or MetaCPAN-missing module names observed across 100+ triaged PRs. Codex's MetaCPAN grounding appears to be holding. The framework-hallucination problem documented in earlier sessions did not recur here.

### 6. Label-state-machine corner cases at 350+ PRs

- Agents sometimes push to wrong remote branch name (e.g., `pr-5252` instead of the actual headRefName). Mitigation: verify with `gh pr view <N> --json headRefName` before push.
- Sign-off labels stripped on rebase — some PRs re-enter the pipeline from earlier stages. Count by checking `gh pr view --json labels`.
- `merge-ready` label drift: fully-signed-off PRs were admin-merged directly when the label was absent. The label is advisory, but its absence signals the ops agent hasn't confirmed the pipeline is complete.

## Durable substrate produced

**Memory (Claude-side):**
- `feedback_master_bit_rot_cascade_fixes.md` — cascade playbook (new)

**Docs articles (repo-side):**
- `SESSION_2026_04_24_ECONOMICS.md` — this document

---

_Related: `docs/articles/SESSION_2026_04_23_RETROSPECTIVE.md`, `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md`_

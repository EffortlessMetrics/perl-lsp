# Session 2026-05-03: ChatGPT Pro ↔ Claude Lean-Loop Learnings

> A working operating model for high-throughput, high-quality semantic-capability work
> using ChatGPT Pro (with GitHub connector) as architect/reviewer and Claude Code as
> implementer, with the user as the routing membrane. Learned across a 4-PR feature chain
> (#7869, #7873, #7880, #7885) on 2026-05-03 that took dynamic strict-bareword diagnostics
> from "fixture-only proof" to "live in production push and pull paths."
>
> This is a session retrospective and a methodology guide. Skip to **Replicable rules**
> if you want the takeaways without the narrative.

---

## What shipped

**Dynamic strict-bareword diagnostic suppression — chain of 4 PRs, all merged 2026-05-03:**

| PR | Title | What landed |
|----|-------|-------------|
| #7869 | `feat(diagnostics): dynamic-boundary suppression infrastructure (PR-A)` | `dynamic_boundary_at` query, semantic-aware scope conversion seam, `DynamicRequire` provenance fix, `NullSemanticQueries` placeholder |
| #7873 | `feat(diagnostics): dynamic callable evidence infrastructure for strict-bareword diagnostics` | `DynamicCallableEvidence` enum (replacing fake `OccurrenceFact { id: 0 }` sentinel), order-aware dynamic-import evidence, literal-eval-sub producer, `Foo->import(@names)` detection, `dynamic_callable_may_be_visible_at` query, `UnquotedBareword` converter extension |
| #7880 | `feat(diagnostics): thread workspace semantic queries into runtime diagnostics` | Runtime wiring at 5 production sites (3 push: `publish_diagnostics`, workspace diagnostic loop, save-triggered push; 2 pull: text/state contexts). New `WorkspaceIndex::with_semantic_queries_for_uri` scoped callback. New `WorkspaceImportExtractor` populating `ImportExportIndex` during `index_file` |
| #7885 | `docs(status): mark dynamic strict-bareword diagnostics as live semantic behavior` | This doc PR's sibling — durable status update marking the chain live |

Plus: issue #7878 (PR-C design archive), issue #7875 (dep-inversion follow-up).

**Live cases in production after #7880:**
- `eval "sub NAME { ... }"; NAME();` → suppresses PL109 for `NAME` only
- `Foo->import(@names);` *then later* bareword → suppresses (order-aware via `ImportSpec.span_start_byte`)
- Bareword *before* `Foo->import(@names);` → still diagnoses (order-aware control)
- No semantic index → legacy fallback unchanged

---

## The operating model that emerged

```
ChatGPT Pro (with GitHub connector)
  → writes architectural briefs + reviews PRs via connector
  → user oversees, pastes briefs/findings between threads
  → Claude Code preflights, implements, pushes PR
  → user takes PR# back to ChatGPT for review
  → user pastes findings to Claude
  → Claude fix-forwards
  → CI green → merge
```

**Five agent spawns per PR, not fifteen.** With ChatGPT Pro doing strategic research/review and the user overseeing, the project's many specialized review agents (`advocatus-diaboli`, `maintainer-issue/pr`, `oppositional-planner`, `architecture-reviewer`, `plan-reviewer`, `research-verifier`, `accuracy-scout`, `refactor-planner`, `green-refactor`, `diff-auditor`) become redundant insurance against single-agent drift that the loop already prevents. Keep `red-tdd`, `green-tdd`, `green-ci`, `pr-responder`, `builder`. Use `reviewer-deep` selectively.

**This is a different machine than the swarm conveyor.** The project's default swarm orchestration excels at backlog draining (50+ PRs, conflict triage, master-health recovery). Capability integration — one feature, one architectural seam, several deeply-related PRs — benefits from continuity instead of fanout. *Pick the machine for the job, not the doctrine.*

---

## Replicable rules

### 1. Mechanical autonomy extends through merge

If a CI break is from your own previous commits AND the fix is **<30 LOC mechanical** (missing field in initializer, unused import, fmt drift, comment alignment, missing release-history entry), don't ask the user. Fix → push → wait CI → merge. The bar is *completely mechanical*: no behavior change, no architectural call, no judgment about scope.

This was hit twice in this session: missing `span_start_byte` field after enum extension, and missing `RELEASE_HISTORY.md` entry after rebase. Both were 1-line fixes. Round-tripping for permission would have wasted 2 user-touches and ~15 min of cache TTL each.

### 2. Cache TTL is the binding constraint, not user time per round

User clock time per copy/paste is ~1 minute. The slow joints are:
- Dead time before user sees a question
- Cache miss past the 1-hour Anthropic prompt cache TTL when Claude goes idle waiting on ChatGPT

**Implication: during ChatGPT review windows, don't go idle.** Preload the next likely move (preflight the next PR, file follow-up issues, tighten tooling, write notes). `ScheduleWakeup` at 7-12 min during CI keeps Claude cheap to resume; 60+ min idle = full context refill.

### 3. Batch architectural questions; never ping mechanical ones

When you have material questions:

```
Preflight finding: <X>
Material questions:
1. <question>
   Why it matters: <reason>
   Proposed default: <pick>
   Consequence if wrong: <risk>
2. ...
I will proceed with the proposed defaults unless you override.
```

For mechanical (path conventions, command variants, fmt reruns, comment alignment, scorecard regen after a fixture change), *just do it and document the choice in the commit message or PR body.* The Q1–Q6 batch on PR-B was the format's first proof — it surfaced a real architectural decision (issue-local vs file-global query) that would have gone wrong silently otherwise.

### 4. State header on every preflight/return

Every PR-related agent return or preflight should open with:

```
origin/master fetched: yes
base SHA: <sha>
branch: <name>
predecessors merged: #X, #Y
PR kind: live behavior / infrastructure / proof / docs / refactor
proof level: 0 stub / 1 workspace query / 2 provider path / 3 runtime/LSP / 4 scorecard
live behavior changed: yes/no
```

This catches the most expensive drift class — stale-base reads, mislabeled scope — at the start of work instead of at PR review. The first PR-B preflight burned ~125k tokens on stale local-master files because this header wasn't enforced; the second preflight (with header) caught the real architectural blocker in 30 minutes.

### 5. Issue-first as design archive

For any non-trivial PR, file the design issue *before* the PR. PR body opens with `Closes #X` / `Plan source: #X`. ChatGPT can then review both archive (intent) and PR (delta). Future LLMs and human reviewers get the same context. Issue #7878 (PR-C design archive) was the first use of this pattern and made the review pass much sharper.

### 6. `cargo check --workspace --all-targets --profile agent --locked` for any PR touching public types

`--lib` alone misses test/bench compile breakage. **Three of four PRs in this chain hit a CI break that `--all-targets` would have caught locally.** Standard from now on for any PR that adds/changes a public type, trait method, fact-schema variant, or cross-crate API.

### 7. Don't store borrowed `WorkspaceSemanticQueries<'a>` — use a scoped callback

`WorkspaceSemanticQueries<'a>` borrows from `Arc<RwLock<...>>` fields of `WorkspaceIndex`. Trying to store it in runtime state was the lifetime constraint that forced PR-C's split into infrastructure + later wiring.

The clean shape:

```rust
workspace.with_semantic_queries_for_uri(uri, |file_id, queries| {
    provider.get_diagnostics_with_path_and_semantics(..., file_id, &queries)
})
```

Holds 3 read guards (lock order: `fact_shards → semantic_reference_index → semantic_import_export_index`) through the closure, releases them when it returns. Verify no nested write-lock acquisition inside the closure or you deadlock.

---

## Anti-patterns surfaced

### "Infrastructure PR" can become a code smell at scale

PR-A and PR-B both retitled to "infrastructure" mid-flight when they hit a structural reason to defer wiring. Each defensible per-PR. By PR-C the cumulative path was 4 PRs for one user-visible behavior. **If you find yourself reaching for the infrastructure framing again on the same feature chain, ask: is the architecture actually ready to support this, or are we incrementally documenting a debt?**

The right next PR may not be more semantic capability — it may be the foundation refactor (e.g., the dep inversion in #7875) that unsticks the rest. After PR-C, two semantic producers (~1100 LOC) live in `crates/perl-workspace/src/semantic/` instead of their architectural home in `perl-semantic-analyzer`, blocked by a `perl-semantic-analyzer → perl-workspace` dep cycle. Each future producer pays the same tax until inverted.

### Don't synthesize sentinel evidence

PR-B initially returned `OccurrenceFact { id: OccurrenceId(0), anchor_id: AnchorId(0) }` as fake "evidence" for the dynamic-import case where no real occurrence existed. Caught in ChatGPT review. **Better: introduce a small enum (`DynamicCallableEvidence` here) or return `bool` than fake a typed value.** Future consumers will eventually inspect those fake IDs.

### ChatGPT operates on stale connector reads

ChatGPT pulls a snapshot when you ask it to review; its mental model lags subsequent commits. Twice in this session ChatGPT flagged "fix this comment" for comments already updated in a more recent commit. **When ChatGPT cites file content, verify against current HEAD before acting.**

### Don't trust the orchestrator's local checkout for code reads

User's local `master` had 3 personal commits not on origin and was ~161 commits behind on origin merges during this session. **Always work in a fresh worktree branched from `origin/master`.** The folder rename (#7866), the dep cycle, and the package-name confusion all became visible only after branching from origin properly.

---

## Codebase-specific gotchas this chain surfaced

These are not in CLAUDE.md and bit at least once during the session:

| Gotcha | Detail |
|---|---|
| **PL109 fires for bare identifiers, not function calls** | `bar()` is parsed as `NodeKind::FunctionCall` and does not currently emit PL109. `print bar;` (bare identifier under `use strict 'subs'`) does. Tests must use the bare-identifier form. Documented on issue #7878. |
| **`WorkspaceSemanticQueries<'a>` lifetime** | Borrows from RwLock-guarded fields. Cannot be stored in runtime state. Use scoped callback (rule #7 above). |
| **Two producers in wrong crate** | `eval_sub_extractor.rs` (415 LOC) + `workspace_import_extractor.rs` (681 LOC) live in `perl-workspace/src/semantic/` because of the `perl-semantic-analyzer → perl-workspace` dep arc. Tracked #7875. |
| **Stale validate-title FAILURE** | The validate-title check runs on title-update events and persists both old (failed) and new (passed) runs in the rollup. `mergeStateStatus: CLEAN/MERGEABLE` is authoritative — if GitHub says CLEAN, the stale FAILURE doesn't block. Same pattern observed across #7860, #7861, #7869, #7873, #7880. |
| **Pre-existing master warnings** | `perl-tdd-support/tests/test_helper_coverage.rs` `unused_must_use`, `perl-incremental-parsing/benches/incremental_parsing_benchmarks.rs` `dead_code`, and `test_dap_build_includes_rs_core_catalog` test failure all exist on `origin/master` already. Don't try to fix in a feature PR. |
| **CodeRabbit + gemini bots are rate-limited per account per hour** | Bot reviews may be delayed by ~1h after a burst of commits. Don't assume bot silence means clean — check explicitly. |

---

## Big-picture insight

> **"exists in isolation" and "actually flows in production" are separated by work nobody wanted to do upfront.**

The semantic spine in this repo was nominally complete months before this session. Every component existed, was named, was tested in isolation. But "exists in isolation" and "actually flows in production" turned out to be 4 PRs of structural plumbing apart for one user-visible behavior. Each step looked like the obvious "make it live" PR; each one hit a real reason to defer. PR-A had a `NullSemanticQueries` placeholder. PR-B couldn't wire production due to lifetimes. PR-C used a scoped callback to solve lifetimes — and discovered cases 1 & 2 needed a *new* producer wired into `index_file`.

**When you see a "ready" architecture, check whether it's actually wired to production.** The wiring is usually where most of the real work lives. Scorecard rows passing is bench alignment; the editor feeling Perl-aware requires the light to travel all the way through the lens.

---

## What's queued next (2026-05-03 EOD)

ChatGPT specs already written, awaiting user go for each:
1. **Real-workspace baseline suite** (`test(semantic): add real-workspace baseline for semantic capability proof`) — current scorecard is 12 deterministic fixtures; need representative project shapes
2. **Receiver-aware method ranking** (`feat(completion): rank method candidates using value-shape-lite receiver hints`)
3. **Foundation cleanup**: dep inversion (#7875) — should land before more semantic producers compound the debt

The next-session orchestrator should resist starting any of these without explicit user confirmation, even though specs are ready.

---

## Cross-references

- Sibling status doc: [`docs/project/status/semantic_capability_dashboard.md`](../project/status/semantic_capability_dashboard.md) — release-readable view of the live dynamic diagnostics
- Architectural doctrine: [`docs/reference/ORCHESTRATION_DOCTRINE.md`](../reference/ORCHESTRATION_DOCTRINE.md)
- Adjacent methodology articles: [`METHODOLOGY_REPLICATION_GUIDE.md`](METHODOLOGY_REPLICATION_GUIDE.md), [`SCORECARD_SEQUENCING_JOURNEY.md`](SCORECARD_SEQUENCING_JOURNEY.md), [`SESSION_2026_04_24_RETROSPECTIVE.md`](SESSION_2026_04_24_RETROSPECTIVE.md)
- Open follow-ups: #7875 (dep inversion), #7878 (PR-C design archive — closed by #7880)

# 2026-06-25 — The Closure Gap: component-proved ≠ system-proved

**Lens**: Why the same defect recurred at every layer of a long orchestrator session, and the invariant that closes it
**Purpose**: Give the next operator/reviewer/agent a named pattern + concrete checks so the closure gap is caught at the seam, before merge/“done”/“live”
**Substrate at time of writing**: Anthropic Claude Code orchestrator (Opus 4.x, Ultracode + long-running warm agents + compaction), ChatGPT-Pro as cold synthesis partner, ~135 workspace crates, perl-lsp-swarm as the live dev repo (frozen `perl-lsp` mirror is the harness session repo), GitHub API intermittently rate-limited (GraphQL exhausted while core healthy)

---

## Problem / what triggered this doc

One ~day-long orchestrator session produced: a retargeted references vertical, an escalated Windows RCE, a 333/349 issue migration, four merged PRs, and ~dozen agent dispatches. **Every consequential defect in it had the same shape** — a component was proved, and the *system* was assumed proved from that. The pattern only becomes visible with the whole session in context. This doc names it, so it is caught next time at the seam instead of after merge.

---

## The core insight

**Failure mode: closure-gap.** A proved component is implicitly taken to prove the integrated system. It is false at every scale, and it is the *same* bug in the product and in the process.

```
component proved  ≠  system proved
```

The needed invariant:

> **No capability is complete until its final consumer, its durable artifact, and its externally observable effect are all verified — bound to the current repo identity and HEAD SHA.**

This is not a metaphor shared between product and process; it is the *same* receipt model. The compiler refuses to emit a semantic fact without identity/provenance/freshness/confidence/source-backing/dynamic-boundary. The control plane should refuse to declare completion without repo-identity/base+head-SHA/artifact-provenance/verification-confidence/production-reachability/known-uncertainty. → cross-links to `compiler-path-forward`, `production-reachability-preflight` memory.

---

## Failure mode: closure-gap — the evidence (all one shape)

Each row: a real, proved part; an unverified chain; the gap that bit.

| Layer | Component proved | Chain that wasn't | What bit |
|------|------------------|-------------------|----------|
| References product | `find_references_with_pir_shadow` unit-tested + re-exported | production `textDocument/references` never *called* it | a "wired/live" vertical that was dark — wrong seam |
| Windows RCE (#2967) | `select_path_candidate`/`resolve_windows_program` hardened, 30 tests green | caller `resolve_command_invocation` does `…unwrap_or_else(|| program.to_string())` → restores the **bare** name → `Command::new` → CWD search | RCE still live behind a "fixed + auto-merge-armed" PR |
| Issue migration | `gh issue transfer` succeeded 333/349 | the issue *list* truncated under API throttle; native/migrated boundary mis-set | 247 native issues mislabeled `migrated` |
| Agent execution | builder edits correct | commit/push/report never happened | "done" work absent from the durable remote |
| Scorecard | routing matrix captured receipts on fixtures | fixtures ≠ real usage distribution | a controlled measurement first described as real usage share |
| CI | the 3 required gates green on the PR | a different lane on `main` was red | a green PR atop a non-green main |

**Rule:** treat a passing component test as evidence about the component *only*. Before "done"/"merge"/"live", verify the chain: caller, reachability, durable artifact, external effect.

**Why:** the gap is always in the **seam the author didn't build** — the caller, the reachability edge, the last-mile handoff, the boundary. Author tests prove what was written; they are structurally blind to what wasn't. Product≈process because the team's blind spot both writes the code and runs the org.

**How to apply:** see the closure receipt and the seam-review division below. Detect by walking one level outward in both directions from any "done" unit: *what feeds this? what consumes this? what happens on None/error/empty?*

---

## Pattern: the multi-axis completion ledger

**Rule:** a single status (`implemented` / `merged` / `feature supported`) is too lossy to mean "done." Track each capability on independent axes; "done" is all of them, not the first.

| Axis | Question |
|------|----------|
| Implemented | Does the code exist? |
| Merged | Is it on current `main`? |
| Reachable | Does a real production request call it? |
| Correct | Does it match an *independent* expected-behavior oracle? |
| Measured | Do representative workloads exercise it? |
| Promoted | Does the user actually receive its answer? |
| Consolidated | Was the superseded path retired? |

**Worked example — the references vertical (as of 2026-06-25):** implemented ✅ · merged ✅ · unit-tested ✅ · production-reachable at the *original* seam ❌ · measured at the *real* seam (newly possible) ◻ · promoted through PIR-A ❌ · legacy overlap retired ❌. A feature-coverage number would have read this as "done"; the ledger reads it honestly.

**Why:** feature catalogs *overstate* usable completion (dark/shadow/wrong-seam/unmeasured implementations count as "supported"); the closure-gap is exactly the distance between the early axes and the late ones.

**How to apply:** when claiming a capability complete, fill all seven axes with evidence. Any ◻/❌ on Reachable/Promoted/Consolidated means inventory, not product.

---

## Pattern: closure receipt (one schema governs product AND process)

**Rule:** bind every completion claim to a receipt carrying identity, SHA, the live entrypoint, chain verification, an independent oracle, remote confirmation, the user-visible effect, and the remaining fallback/uncertainty.

```yaml
repo: EffortlessMetrics/perl-lsp-swarm
base_sha: <merge-base>
head_sha: <verified HEAD>
issue: <#>
pr: <#>
claim: <one sentence>
production_entrypoint: <the real request handler that reaches this>
call_chain_verified: true            # walked caller→callee, incl. None/err/empty paths
tests:
  - command: <cmd>
    result: pass
independent_expected_behavior: <oracle, not the author's own test>
remote_head_confirmed: true          # verified on origin, not the agent's word
user_visible_effect: <what the user now observes>
fallback_remaining: <legacy path still live?>
uncertainty: <what is NOT proved>
```

**Why:** this receipt, populated honestly, would have blocked the references "live" claim (`production_entrypoint`/`call_chain_verified` fail), the RCE auto-merge (`call_chain_verified` fails on the None path), the dropped-commit "done" (`remote_head_confirmed` fails), and the scorecard overclaim (`independent_expected_behavior`/usage qualifier).

**How to apply:** require it for any security/correctness/"live" claim before merge. It is the process-side instance of the compiler's fact-with-provenance and the ub-review evidence gate. → cross-link `ub-review-tool`, `control-plane-is-the-binding-constraint`.

---

## Pattern: adversarial review belongs at the seam, not in the function

**Rule:** cold/oppositional review's primary job is to walk **one level outward in both directions** from the change — not to re-read the changed function. The consequential defects this session all lived at seams the author didn't model: selector→invocation, helper→handler, intended→observed index-state, migrated-issue→provenance-label, local-worktree→remote-PR, fixture-distribution→user-distribution.

Three-role division of labor:

```
warm owner       — build + iterate within the component
cold/redirected  — inspect assumptions, callers, fallbacks, boundaries (one level out, both ways)
fresh repair     — fix one narrow defect when warm context has degraded
```

**Why — and the session's highest-leverage move:** a **re-tasked warm agent** caught the references dark-wrapper and a fresh full-context read caught the RCE fail-open caller. The value of cold review is the **different direction**, not clean context — you do not need a fresh agent, you need a different approach pointed at the seam. Re-pointing an idle-but-warm agent at "review your own lane's callers and None-paths" is cheap and catches what the build pass cannot see.

**How to apply:** for any consequential change, run one pass whose *only* questions are: what feeds this? what consumes this? what happens when it returns None/Err/empty? Default that pass to "refuted unless proven." Prefer re-pointing a warm agent over spawning cold when context is still hot.

**Boundary:** mechanical facts (file paths, fmt, clippy, gate config) belong to automated guards, not the cold reviewer — don't spend judgment tokens on greppable truth.

---

## Pattern: the backlog is a completion harvest (inverse accounting)

**Rule:** the issue tracker *understates* completion while the feature catalog *overstates* usable completion. The open backlog is not N unsolved problems; it is an inventory of: already-implemented · stranded-implementation · small-remaining-delta · duplicate-research-history · obsolete-premise · real-larger-project. Disposition each with evidence; do not auto-build.

**Why:** old issues carry prior-PR links, exact files, tests, and receipts — significant already-paid-for engineering never reconciled into closed state. The duplicate PRs #2666/#2669 are the same pattern at PR scale. → cross-link `migrated-backlog-completion-harvest`.

**How to apply:** run a harvest lane per subsystem (haiku verifies current-main + does PR archaeology; sonnet only to reconstruct a patch). One disposition per issue: DONE (post SHA evidence, close) · STRANDED-PATCH (rebase/repair → small PR) · SMALL-DELTA (narrow PR) · DUP/SUPERSEDED (cross-link, close) · REAL (retain with a current-main boundary). Pilot 40–50 across 3 clusters, track real yield, then scale.

---

## Pattern: three inventories (file aggressively without faking an execution queue)

**Rule:** keep the research graph, the execution queue, and the product-closure queue as **distinct** lists with distinct trust levels. Filing into the first is ~free; promotion between them costs evidence.

```
research graph      cheap, broad, preserved   { migrated-unverified, suspected, historical, dup-linked, already-fixed-backlink }
execution queue     small, expensive          { verified-current-main, production-reachable, owned, builder-ready, ranked }
product closure     smaller still             { merged, reachable, measured, promoted, legacy-overlap-awaiting-removal }
```

**Why:** this is what lets "file aggressively" coexist with "never spawn a builder from unverified intake." A migrated issue is research-graph until a current-main repro promotes it. Conflating the lists is what produced builds off stale premises and the mislabel cascade.

**How to apply:** label by inventory; gate promotion on a current-main verification receipt. → cross-link `file-issues-aggressively`, `migrated-backlog-completion-harvest`.

---

## Pattern: the bottleneck migrates upward — infra IS product velocity

**Rule:** as implementation gets cheaper, the binding constraint moves up the stack. Optimize the *current* constraint, not codegen.

```
code generation → compilation → CI evidence → merge policy → GitHub API → issue reconciliation → reviewer attention
```

**Why:** this session became limited by GraphQL rate limits and classification throughput, not the ability to write Rust. When codegen is free, every artifact that doesn't convert to durable product state is inventory, and the conversion chain (finding → verified-current-main → owned → pushed → merged → reachable → measured → legacy-retired) is the actual product line. → cross-link `control-plane-is-the-binding-constraint` (the constraint doesn't just exist; it *moves*).

**How to apply:** spend infra effort on the live constraint: cached builds (sccache), fewer duplicate branches/PRs, idempotent bulk operations, durable agent-completion receipts, current-main preflight, API-aware write queues, representative evidence over repeated CI cycles. Treat these as product work, not support work.

---

## Pattern: every proxy carries its claim boundary

**Rule:** annotate each metric/receipt with what it does **not** prove. Proxies were useful here and then overclaimed.

| Proxy | Proves | Does NOT prove |
|-------|--------|----------------|
| Patch coverage (Codecov 95) | changed lines executed | semantic correctness |
| RIPR exposure | tests strongly expose a seam | the whole provider is correct |
| Controlled routing matrix | routing + receipt capture work | real usage share |
| Shadow comparison | candidate vs current path compared | users receive the candidate |
| Unit test | component contract holds | production call chain reaches it |
| Clean parse | syntax accepted | semantic truth |
| Merged PR | code landed | user benefit |

**Why:** the closure-gap hides in the right-hand column. Each overclaim this session was a proxy read as if it proved the next axis.

**How to apply:** when citing a proxy as evidence, state its boundary in the same breath. This is the same discipline the snapshot/oracle/scorecard work is already converging toward.

---

## Pattern: cache is a discount, not a priority system

**Rule:** an idle-but-warm agent (5-min cache vs ~1-hr orchestrator) is cheap context to re-task — but cache preservation must **not** manufacture low-value work.

**Why:** re-tasking onto adversarial seam-review / test-hardening / current-main verification / reconciliation is high-leverage *because the work was already valuable*; the warm cache only makes it cheaper. Inventing busywork to "use the cache" inverts the priority. → cross-link `re-task-idle-warm-agents`, `warm-agent-reliability-patterns`.

**How to apply:** on an idle ping, re-point the warm agent at the highest-value adjacent work in priority order (adversarial review of its own lane → test strengthening → docs/cleanup → adjacent current-main verification → issue reconciliation → only then unrelated). If none clears the bar, let it expire.

---

## Pattern selection is not pipeline-vs-agent — pick per independence

**Rule:** the old state-machine pipeline and modern long-running lanes are not opposites. Choose by where independence is load-bearing.

```
warm lane        owns continuity
cold gates       own independent judgment
Ultracode        supplies breadth + opposition
mechanical guards own auto-checkable facts
```

**Why:** the pipeline was built when context was expensive/fragile; long steerable agents + compaction + Ultracode change the optimal unit of ownership. The mistake is selecting either *reflexively*. → cross-link `prefer-consolidated-long-agents-over-granular-pipeline`.

**How to apply:** keep gate *intent* (the independent judgment), consolidate agents where continuity dominates, fan out where independence dominates.

---

## How to apply (operator checklist)

Before declaring any consequential change done/live/merged:

1. **Reachable?** Name the production entrypoint and walk caller→callee to the change. Confirm a real request reaches it.
2. **Fail-closed at the glue?** For every `unwrap_or_else(|| original)` / fallback on the chain, confirm the None/Err path does not restore unsafe/stale input.
3. **Durable?** Verify the artifact on `origin` (remote HEAD SHA), not the agent's word.
4. **Independent oracle?** Correctness checked against something other than the author's own test.
5. **External effect?** State what the user now observes.
6. **Legacy retired?** Note the superseded path and its removal plan.
7. **Proxy boundaries?** Every metric cited with what it doesn't prove.

Missing any of 1–6 ⇒ inventory, not product. Run the seam-review pass (one level out, both directions) by a redirected-warm or cold reviewer; mechanical facts to automated guards.

---

## Failure modes / when this doc's rules don't apply

- **Trivial mechanical changes** (fmt, a one-line doc fix) don't need the full closure receipt — the chain is the change.
- **The closure receipt is itself a proxy**: a populated receipt with a dishonest `call_chain_verified: true` proves nothing. The receipt disciplines the *questions*; an adversary must still drive the real chain.
- **Cache re-tasking** must not override the priority order (see that pattern's boundary).

---

## Related forensics + memory entries

- `production-reachability-preflight` (memory) — prove a real JSON-RPC request reaches the code before "live"; the runtime complement is a tier scorecard
- `verify-whole-chain-not-component` (memory) — the closure-gap as the session's #1 recurring bug, with the four product/process instances
- `migrated-backlog-completion-harvest` (memory) — the harvest lane + dispositions + `migrated ⟺ swarm# ≥ 2675` governance
- `canonicalize-fallback-fail-open-bypass` (memory) — fail-closed at the glue; the RCE fail-open caller as exemplar
- `control-plane-is-the-binding-constraint` (memory) — the bottleneck migrates upward; infra is product velocity
- `re-task-idle-warm-agents` / `warm-agent-reliability-patterns` (memory) — cache is a discount; verify the durable remote not the agent
- `ub-review-tool` (memory) — the closure receipt productized as a fail-closed PR evidence gate
- `2026-04-25-defense-in-depth-verification-architecture.md` — verifier-ladder structures this composes with
- `2026-04-25-failure-mode-catalog.md` — register `failure mode: closure-gap` alongside the existing catalog

---

## Applies to

Loaded into prompt context for: **reviewer-deep** / **diff-auditor** / **architecture-reviewer** (seam-review one-level-out is their job), **scout**/**plan-reviewer** (multi-axis ledger when scoping "is this done?"), **green-ci**/**ops** (closure receipt before merge; main-green not just PR-green), **wisdom**/**memory-recalibrator** (completion-harvest + three-inventories), and any orchestrator session that ends with "is this actually closed?"

The situation_id this doc serves: any moment where a proved component is about to be reported as a working system.

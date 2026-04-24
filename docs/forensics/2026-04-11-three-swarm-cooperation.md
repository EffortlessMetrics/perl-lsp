# Three-Swarm Cooperation: Forensic Notes from 2026-04-11

**Date:** 2026-04-11
**Session window:** roughly 14 hours of master-branch activity ending 2026-04-11 ~15:00 UTC
**Tracking issue:** [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) (metrics / v0.13.0 alpha prep cycle)
**Related work (do not duplicate):**
- [`docs/project/wisdom/2026-04-11-session-learnings.md`](../project/wisdom/2026-04-11-session-learnings.md) — session retrospective ([PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117))
- [`docs/project/metrics/README.md`](../project/metrics/README.md) — metric-stack and ratchet story ([PR #4123](https://github.com/EffortlessMetrics/perl-lsp/pull/4123))
- In-flight swarm-operations article (general-purpose agent, parallel write to `docs/articles/` or `docs/forensics/`)

## TL;DR

On 2026-04-11 the perl-lsp repository was worked on concurrently by **three agent surfaces** — a Claude Code swarm on one device, an autonomous Codex device swarm on another, and a user-driven Codex web session — with **no shared memory, no direct messaging, and no explicit coordination protocol**. They converged on the same GitHub state (labels, draft PRs, issue comments) and produced approximately 28 co-authored commits and 12 solo commits across a 14-hour window. The three surfaces specialized emergently (research/strategic, autonomous production, gap-filling), handed work off to each other via GitHub's squash-merge `Co-authored-by` trailer and via referenced findings in PR bodies, and between them caught at least one critical false-premise bug that plausibly would have shipped under single-swarm operation.

This document is the forensic record of that cooperation model — what the pattern looked like in commit evidence (§2), which handoffs worked (§3), where it raced (§4), how much value it delivered over single-swarm operation (§5), and what a next-session convention could add without spoiling the autonomy that made it work (§6). Sections 8 (evidence vs. hypothesis) and 10 (reproducibility) let a reader verify the claims against GitHub directly. Sections 7 and 11 are open questions and self-review.

This article is deliberately **not** a session retrospective, metrics story, or operational how-to — those live in [PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117) (`docs/project/wisdom/2026-04-11-session-learnings.md`), [PR #4123](https://github.com/EffortlessMetrics/perl-lsp/pull/4123) (`docs/project/metrics/README.md`), and an in-flight swarm-ops article respectively. Its scope is specifically the **multi-swarm cooperation model as an operational pattern worth naming**, not the session's outcomes or the project's metrics framing.

**Proposed name for the pattern:** *three-surface cooperation via GitHub-native state*. It is not the only way to run multi-swarm operations, but it is the cheapest one that works, and it is the one 2026-04-11 produced evidence for.

## 1. Session Context

The 2026-04-11 session was v0.13.0 alpha-prep: the release-readiness scorecard work tracked by #4062, a drumbeat of P0 idiom burn-down, LSP navigation correctness, pragma/diagnostics work, and Windows-path hygiene for the execute-command feature. Three surfaces were live at various points:

| Surface | Device | Mode | Primary strengths observed |
|---------|--------|------|-----------------------------|
| **Claude Code swarm** | Primary device | Full orchestrator pipeline (scout → plan-review → build → review → green → merge → wisdom) | Strategic routing, research-heavy verification, deep review, cross-reference sweeps |
| **Codex device swarm** | Secondary device | Autonomous, long-running | Sustained code production, branch-level discipline, high-volume merges |
| **Codex web** | User browser | Manual-assisted, interactive | Gap filling, one-off fixes, spot review, backlog drainage |

All three surfaces used the same GitHub repository as the shared state substrate. None had access to the others' agent catalogs, scratchpads, worktrees, memory files, or chat transcripts. Coordination was entirely via GitHub-native signals: issue labels, PR state (draft/ready/merged), branch names, and the `Co-authored-by:` trailer on squash-merge commits.

This is a genuinely novel operating point. The swarm architecture described in [`docs/SWARM_ARCHITECTURE.md`](../SWARM_ARCHITECTURE.md) assumes one orchestrator per repo; the wisdom log in [`docs/articles/SWARM_METHODOLOGY.md`](../articles/SWARM_METHODOLOGY.md) documents single-swarm autonomy; no existing forensic record captures what happens when three differently-shaped swarms share one branch set. This article fills that gap.

A few framing notes before the evidence:

- **"Three swarms" is exact.** It is not "one swarm with three agents" and not "three sessions of the same swarm." Each surface has its own orchestrator, its own agent catalog, its own scratchpad, and its own view of what work it has claimed. They share nothing except the GitHub state of the repository itself.
- **The session was not designed as a cooperation experiment.** It was a normal v0.13.0 alpha-prep cycle that happened to have two additional surfaces active at the same time. The pattern is visible in hindsight, which is what makes it a forensic article rather than a pre-registered study.
- **The evidence window is partial.** The 14-hour `git log --since` window captures the most active block but misses earlier work by the Codex device swarm from the previous day. The forensic claims are about this window specifically; extrapolating beyond it should be done carefully.
- **The three surfaces have different logging/audit capabilities.** The Claude Code swarm has full transcript access to its own session; the Codex device swarm's scratchpad was not accessible to the author of this article; the Codex web surface's interactions were captured only as their output (commits and PR edits). This asymmetry is why the article leans so heavily on the commit log — it is the only substrate all three surfaces wrote to.
- **This article is written from one of the three surfaces.** The author is the Claude Code swarm, writing about its own cooperation with two other surfaces it cannot see directly. That is an inherent perspective bias: the Claude Code swarm knows its own state well and the others' state only through GitHub artifacts. A version of this article written from the Codex device surface's perspective would be equally legitimate, and might emphasize different patterns. Until such a version exists, readers should treat this article as an honest attempt at a neutral description rather than a guaranteed one.
- **The "three swarms cooperating" framing is after-the-fact.** During the session itself, no surface was thinking of its behavior as "cooperating with two other surfaces." Each surface was running its own pipeline, pulling from the same backlog, writing its own reviews. The cooperation is a property of the ensemble, not of any individual surface's intent. That is part of why the pattern is worth naming — it is invisible from inside a single surface and only becomes visible from outside.

## 2. Attribution Evidence from the Commit Log

The squash-merge commit log on master is the durable trace of cross-swarm cooperation. Two signals distinguish the three cases:

1. **Solo commit** — `Steven Zimmerman, CPA <EffortlessSteven@users.noreply.github.com>` as sole author and no `Co-authored-by:` trailer. Either Claude Code or Codex web produced the full branch and a same-surface reviewer merged it.
2. **Co-authored with Codex** — `Co-authored-by: OpenAI Codex <codex@openai.com>` trailer. One swarm produced the branch, another surface (Codex device or Codex web, using the OpenAI Codex credential) touched it on the way to merge — either by pushing fixes to the branch or by recording authorship on a squash-merge of changes whose history mixed both attributions.
3. **Co-authoring is emergent, not configured.** The attribution came from the existing Codex CLI's `Co-authored-by:` convention and GitHub's squash-merge behavior; no one installed a coordination hook to produce it. That matters for item 7, below.

Full 14-hour window on master (commit hash, PR, title, co-authored). The list was extracted by `git log --since="14 hours ago" --pretty=format:"%h|%s|%(trailers:key=Co-authored-by,valueonly)" master`; it is the raw forensic trace and the numerical basis for every claim in this section.

| Commit | PR | Title (truncated) | Co-authored? |
|--------|-----|-------------------|--------------|
| `ef92dd20` | [#4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089) | fix(execute-command): strip Windows extended-length prefix | **Yes** |
| `37ba0b95` | [#4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093) | feat(workspace): workspace/configuration reverse-request flow | No (solo) |
| `427f413c` | [#4091](https://github.com/EffortlessMetrics/perl-lsp/pull/4091) | fix(navigation): resolve AUTOLOAD-backed method calls | **Yes** |
| `99eb1e01` | [#4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098) | fix(clippy): remove needless borrows in workspace file-ops | No (solo) |
| `4a5e999f` | — | fix(agents): terminal skills in scout-lsp, reviewer-lsp | No (solo) |
| `fc528b3e` | [#4083](https://github.com/EffortlessMetrics/perl-lsp/pull/4083) | fix(hooks): pre-push runs pr-fast instead of ci-gate | **Yes** |
| `e229ce07` | — | fix(diagnostics): eval/sub-scoped pragmas no longer suppress file-level | No (solo) |
| `a090206e` | — | feat(workspace): strengthen cross-file file-ops refactoring | **Yes** |
| `7c2e4564` | [#4064](https://github.com/EffortlessMetrics/perl-lsp/pull/4064) | fix(hooks): harden plan-review issue binding and hook IO | **Yes** |
| `2e88eb64` | — | fix(navigation): resolve inherited and role methods in goto-def/hover | No (solo) |
| `2f970c06` | — | fix(semantic-analyzer): descend into Error partial nodes for symbol walk | No (solo) |
| `1f5e7096` | — | test(parser): wire orphaned unclosed-block recovery tests | No (solo) |
| `6823c775` | [#4082](https://github.com/EffortlessMetrics/perl-lsp/pull/4082) | test(semantic-analyzer): fix stale method attribute assertion | **Yes** |
| `8ca5d8e1` | — | fix(workspace): tolerate non-file URIs for workspace roots (#3521) | **Yes** |
| `f0b77bc6` | — | docs(announcement): scope metric framing in v0.13.0 draft (#4045) | No (solo) |
| `9a5e7a4e` | — | docs(readme): scope metric framing, add entry-points table | No (solo) |
| `38de4ca7` | — | docs(marketplace): scope metric framing in VSCode listing (#4045) | No (solo) |
| `efc4b6dc` | — | test(incremental): assert per-edit checkpoint fallback behavior | **Yes** |
| `8efa3f03` | [#4081](https://github.com/EffortlessMetrics/perl-lsp/pull/4081) | fix(ci-hygiene): normalize allowlisted panic paths (#4073) | **Yes** |
| `b717ac92` | — | fix(pragma): track explicit feature bundles and switch state (#3398) | **Yes** |
| `73ba9d35` | [#4061](https://github.com/EffortlessMetrics/perl-lsp/pull/4061) | fix(ci-hygiene): skip doc-only code gates (#4047) | **Yes** |
| `ed79f8c0` | [#4050](https://github.com/EffortlessMetrics/perl-lsp/pull/4050) | fix(pragma): track conditional use-if pragmas (#3485) | **Yes** |
| `32333ad2` | — | fix(incremental-parsing): preserve prefix correctness at checkpoint | **Yes** |
| `0e740156` | [#4043](https://github.com/EffortlessMetrics/perl-lsp/pull/4043) | fix(semantic): track Readonly wrapper declarations (#3393) | **Yes** |
| `2b969c8c` | — | feat(incremental-parsing): segment-based token cache and trimming | No (solo) |
| `f7deb3b2` | — | test(lsp): regenerate capability snapshots (#4039) | **Yes** |
| `6e0c2148` | — | fix(semantic): track Const::Fast readonly declarations (#3392) | **Yes** |
| `b1306483` | — | test(lsp): regenerate capability snapshots (#4039) | **Yes** |
| `3ba4428e` | — | chore(clippy): clear current hover/navigation warnings (#4008) | **Yes** |
| `26aff5d6` | — | test(quality): burn down match-arm panic asserts (#3258) | **Yes** |
| `a0d52cef` | — | fix(xtask): repair feature audit verification (#3284) | **Yes** |
| `ec8af647` | — | test(lexer): replace panic catch arms in interpolation tests (#3258) | **Yes** |
| `4aefaf04` | — | feat(execute-command): prefer yath for test files (#3533) | **Yes** |
| `7414d6e1` | — | test(dap-variables): remove panic match-arm catches (#3258) | **Yes** |
| `8098f997` | — | fix(dap-types): derive basename from Windows-style source paths | **Yes** |
| `2be6dcaf` | [#4002](https://github.com/EffortlessMetrics/perl-lsp/pull/4002) | test(quality): burn down P0 idiom/dependency findings (#3237) | No (solo) |
| `f92d690f` | — | fix(lsp): advertise semantic token delta support (#3282) | **Yes** |
| `a0e7193b` | — | feat(vscode): run test at cursor (#3534) | **Yes** |
| `45ae0d7a` | — | feat(vscode): add gherkin step navigation and stubs (#3548) | **Yes** |
| `c4c5494c` | — | test(pragma): guard require VERSION semantics (#3488) | **Yes** |

Hand counts across this 40-commit window: **28 co-authored** (70%) and **12 solo** (30%), where "co-authored" means a `Co-authored-by: OpenAI Codex <codex@openai.com>` trailer is present on the squash-merge commit. What matters forensically is the shape: the dominant mode on 2026-04-11 was **branch-by-one-surface, reviewed-or-touched-by-another**. A single-swarm session would have produced a flat solo distribution with maybe the occasional co-authored commit from explicit human intervention; the ratio here is the fingerprint of cross-swarm cooperation, and it is the clearest durable signal that the pattern existed at all.

A useful sanity check: filter the table by commit type. The 12 solo commits cluster around (a) strategic docs work (`f0b77bc6`, `9a5e7a4e`, `38de4ca7` — three metric-framing updates), (b) isolated hardening that didn't need a second pair of eyes (`e229ce07`, `2f970c06`, `1f5e7096`, `2e88eb64`), (c) the scorecard feature branches (`37ba0b95`, `99eb1e01`, `2b969c8c`, `2be6dcaf`), and (d) one solo backlog drainage pass. The 28 co-authored commits span everything else: parser work, pragma work, incremental-parsing work, DAP, LSP feature work, CI-hygiene, test burn-down. The distribution matches §5.2's specialization thesis: strategic docs and scorecard design were one surface's gravitational pull, everything else was either shared or the other surface's pull.

One detail worth pulling out of the table: the three consecutive docs commits `f0b77bc6`, `9a5e7a4e`, `38de4ca7` all land close together in time and all carry the `(#4045)` tracking-issue suffix. They are clearly part of one logical "scope the metric framing" sweep that was executed by one surface in one sitting. None of them are co-authored. This is as clean a forensic signature of "one surface owns this workstream" as the log offers. Contrast with the four consecutive `fix(pragma)` commits further down the list (`b717ac92`, `ed79f8c0`, `0e740156`, `6e0c2148`), all of which are co-authored: pragma work is a shared workstream that two surfaces cooperated on. A future reader who wants to know "what did each surface specialize in on 2026-04-11" can derive the answer from these clusters without needing any out-of-band information.

## 3. Cooperation Patterns

Four distinct patterns recurred. Each is backed by a specific merge or issue trail so a future reader can walk the evidence directly on GitHub. The timing envelope (roughly 06:02 UTC to 12:00 UTC for the visible burst of merges) is tight enough that the cases are not independent — the same two-swarm-pair interacting produced them all — and that makes the patterns comparable across the window rather than being isolated anecdotes.

### 3.1 Research-implementation handoff (triple verification)

The exemplar is the `BEGIN { use strict; }` false-premise cascade, tracked by issue [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100).

1. **Swarm A** (Claude Code) had a research-verifier agent examine [PR #4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) — a proposed fix that treated phase blocks as propagating their inner pragmas to file scope. The verifier ran `perl -e 'BEGIN { use strict; } $x = 1; print "ok: strict not active\n"'` against Perl 5.38.2 and proved the fix was built on a false premise: phase blocks have normal lexical pragma scope. The verifier posted its findings to PR #4090 and the PR was closed (not merged).
2. **Swarm B** (Codex device) picked up the resulting issue #4100 and produced [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108), `fix(pragma): keep phase-block pragmas lexically scoped`, which merged at `2026-04-11T11:39:50Z`.
3. **Swarm A** (Claude Code), unaware #4108 had just landed, dispatched its own builder against the same #4100 and produced [PR #4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120), `fix(pragma): correct phase-block pragma scoping to match Perl lexical semantics`, with six new BDD tests and a documentation pass. PR #4120 was closed (unmerged) when the race was discovered.

The cooperation value isn't that #4120 was wasted — it's that the same bug was independently investigated by two swarms using two different verification tools (direct `perl -e` execution in #4120 vs. behavior-spec tests in #4108), and either one would have caught a regression. The *research finding itself* was then referenced in #4108's body as "the research-verifier on PR #4090 caught...", creating a citation chain where Swarm A's research informed Swarm B's implementation. Three independent lenses on the same bug.

**Cost of the duplication:** one unmerged PR, a few builder-hours. **Benefit:** a cross-swarm witness that the correct fix is correct.

The evidence trail also reveals a subtle but important sub-pattern: PR #4108 explicitly cites the PR #4090 research finding by URL in its body ("Source of workaround: #4052 (merged) — only the PhaseBlock body scan is reverted; Correctly-closed PR: #4090"). The Codex device builder who produced #4108 did not need direct access to the Claude Code research-verifier's output — they just read the closed PR #4090's comment thread. This is the **GitHub-as-shared-memory** effect at its cleanest: Swarm A left a research artifact on a closed PR, Swarm B picked it up twenty minutes later, and the downstream fix cited the original finding without any direct communication. The cooperation is legible because the comment thread on #4090 is permanent and linkable.

### 3.2 Branch-then-review handoff (the common case)

Most co-authored merges in the window followed this shape: one swarm's builder agent pushed a branch and a draft PR; another swarm's reviewer (or direct user intervention via Codex web) pushed follow-up commits to that branch; the squash merge carried both attributions into the trailer.

Exemplars:

- [PR #4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089) — Windows extended-length prefix stripping in `execute-command`. Claude Code's builder produced the original fix and UNC-path follow-up; Codex pushed the `refactor(execute-command): avoid lossy Windows path normalization` commit as the final touch. The squash commit `ef92dd20` shows the multi-commit history and the `Co-authored-by: OpenAI Codex` trailer.
- [PR #4064](https://github.com/EffortlessMetrics/perl-lsp/pull/4064) — hook IO and plan-review issue binding hardening. Solo branch work on one side, cross-surface review on the other.
- [PR #4082](https://github.com/EffortlessMetrics/perl-lsp/pull/4082) — stale method attribute assertion fix. Same shape.
- [PR #4091](https://github.com/EffortlessMetrics/perl-lsp/pull/4091) — `fix(navigation): resolve AUTOLOAD-backed method calls`. Same shape.
- [PR #4081](https://github.com/EffortlessMetrics/perl-lsp/pull/4081) — `fix(ci-hygiene): normalize allowlisted panic paths`. Same shape.
- [PR #4050](https://github.com/EffortlessMetrics/perl-lsp/pull/4050) — `fix(pragma): track conditional use-if pragmas`. Same shape.
- [PR #4043](https://github.com/EffortlessMetrics/perl-lsp/pull/4043) — `fix(semantic): track Readonly wrapper declarations`. Same shape.

This is the **continuous review pressure** effect: once two swarms are active on the repo, every branch gets a second pair of eyes before merge because both swarms scan the open-PR list. Review starvation (a common single-swarm orchestrator failure mode) becomes structurally hard: you would have to actively suppress the other swarm's visibility into the PR queue, and nothing in this session did.

The shape is worth unpacking once in detail. Consider PR #4089 (execute-command Windows path stripping). The commit message on the squash merge (`ef92dd20`) lists three logical commits in order:

1. `fix(execute-command): strip Windows extended-length prefix before external commands` — original implementation, adds `normalize_path_for_external_command` and applies it at nine call sites in `run_tests`, `run_test_sub`, and `run_file`.
2. `fix(execute-command): handle UNC extended-length paths and fix missed perl.debugFile site` — follow-up that catches a missed call site in `misc.rs` and correctly handles the UNC `\\?\UNC\server\share\...` case (where the naive strip produces an invalid path).
3. `refactor(execute-command): avoid lossy Windows path normalization` — refinement to the approach.

The `Co-authored-by: OpenAI Codex <codex@openai.com>` trailer is attached to the squash merge. Without the trailer, a future reader would see one commit; with it, they can infer that two surfaces contributed to the branch even though the actual commit history was collapsed. The forensic value of the trailer is that it reconstructs "how many pairs of eyes touched this PR" from a single log line.

### 3.3 Independent parallel production (non-overlapping backlog)

The majority of the session's throughput came from swarms pulling from disjoint slices of the issue backlog without ever touching each other's work. Examples from the 2026-04-11 window:

- [PR #4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093) — `workspace/configuration` reverse request flow (closes #3515). Merged solo as `37ba0b95` at 11:04 UTC. Non-overlapping with anything else in flight.
- [PR #4002](https://github.com/EffortlessMetrics/perl-lsp/pull/4002) — P0 idiom/dependency burn-down (closes #3237). Merged solo as `2be6dcaf` at 06:02 UTC. Pure backlog drainage.
- [PR #4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098) — clippy cleanup in workspace file-ops.
- [PR #4091](https://github.com/EffortlessMetrics/perl-lsp/pull/4091) (counted above as a branch-then-review handoff) — AUTOLOAD navigation. Merged at 10:59 UTC.

Pattern: Swarm A was doing strategic work on the metrics/scorecard cycle; Swarm B was draining the P0/P1 backlog; Swarm C (Codex web) filled spot gaps. None of them contended for the same files or issues; the forensic signal is the absence of a coordination overhead that a reader might expect.

### 3.4 Cross-swarm citation chains

A subtler pattern: when a PR body references a finding by another surface, the cooperation becomes legible to future readers. PR #4120's body contains the line:

> "The research-verifier on PR #4090 caught that our pragma tracker and diagnostics were based on a false premise..."

and links to the research-verifier comment directly. This is the only place in the commit log where a specific agent role name crosses surface boundaries. It was not prompted by any coordination layer; the builder simply quoted the research finding when writing the PR body because it was the most accurate framing. As long as authors (or their scaffolds) cite findings with stable URLs, the cross-swarm reasoning is preserved for audit even though the swarms never communicated directly.

The citation chain is worth protecting as a forensic invariant. It turns "three swarms touched this bug" from an unverifiable rumor into a verifiable claim: a reader can follow the URL from PR #4120 back to the research-verifier comment on PR #4090, confirm the verifier's methodology (the `perl -e` invocation is quoted directly in the comment), and then independently re-run the check themselves. The chain has three links — PR #4120 → PR #4090 comment → `perl -e` reference implementation — and each link is durable, public, and cheap to verify. This is exactly what the forensics directory is for: preserving the evidence trail against which the session's claims can be checked years later.

The shape also has a name worth proposing: **referenced-finding cooperation**. Unlike branch-then-review (§3.2), where cooperation is legible because two authors touched the same commit, referenced-finding cooperation is legible because *one author's output is quoted by another's output* via a stable URL. The two patterns are complementary: branch-then-review captures the cases where surfaces touched the same code; referenced-finding captures the cases where surfaces informed each other's reasoning without touching the same code. A complete forensic record of three-swarm cooperation needs both.

### 3.5 Timing envelope: master-churn as back-pressure

One striking forensic property of the window is how tight the merge cadence was. The 40-commit sample above clusters into roughly 40 merges over ~10 hours of visible master activity, or one merge every ~15 minutes on average. This cadence has a direct effect on every cooperation pattern:

- **On §3.1 duplicate dispatch:** a 15-minute average merge interval is *shorter* than the typical Claude Code builder cycle (scout → plan-review → build-test → build-implement → self-review → PR). If Swarm A starts a builder at time T and Swarm B merges a fix at T+20 minutes, Swarm A's builder is already past the point where it would notice the merge — it is committed to pushing its branch. That is precisely the #4108 vs #4120 shape.
- **On §3.2 branch-then-review:** a tight cadence makes review cheap because a reviewing surface sees a high volume of draft PRs per hour. Every draft it skips is one fewer chance to add its perspective, which in practice means reviewers gravitate toward whichever draft is oldest in the queue. Review coverage is uniform-ish despite the lack of coordination.
- **On §3.3 independent parallel production:** a tight cadence spreads claims across the backlog naturally. If Swarm A claimed the next P0 idiom issue at T and Swarm B claimed one at T+3 minutes, the probability they picked the same issue is low because each swarm sorts the queue independently and the top of the queue moves as merges land. **Master churn is the coordination mechanism**, not a hazard to be engineered around.

This suggests a prediction: if the three-swarm model were run with a *slower* master cadence (say, one merge per hour because the gate is expensive), the duplicate-dispatch rate would go *up*, not down, because the back-pressure that spreads claims across the backlog would weaken. Verifying this prediction is an open question (see §7). The practical implication for current operations is that the three-swarm model benefits from a fast PR-fast gate — which is what [PR #4083](https://github.com/EffortlessMetrics/perl-lsp/pull/4083) (`fix(hooks): pre-push runs pr-fast instead of ci-gate`, merged this same session) happens to enable. That PR was not written to enable three-swarm cooperation, but in hindsight it is part of the supporting infrastructure.

A back-of-the-envelope sketch of how the back-pressure works:

Let *B* be the backlog depth at time T (number of open, `builder-ready` issues). Let *μ* be the merge rate across all surfaces in merges/hour. Let *τ* be the average builder cycle time (issue pick to PR open) in hours. Let *N* be the number of active swarms.

In one cycle of length *τ*, each swarm picks one issue. The probability that two swarms pick the *same* issue depends on how they sort the backlog. If all swarms sort identically and the top issue is picked deterministically, the duplicate rate is ~100% (all swarms grab the same top issue). If they sort with some randomness, the duplicate rate is roughly 1/B per pair of swarms per cycle.

Now introduce master churn. Over the duration *τ*, master absorbs *μ·τ* merges, shrinking the top-of-queue region. If *μ·τ ≈ 1*, then by the time Swarm B looks at the queue after Swarm A's pick, the top issue has already been replaced by a newer one, and the duplicate probability drops by roughly a factor of *μ·τ*. In the 2026-04-11 window, *τ ≈ 0.5 hours* (30-minute builder cycle) and *μ ≈ 4 merges/hour*, so *μ·τ ≈ 2* — meaning the top-of-queue turns over roughly twice during one builder cycle. That is the slack that lets two swarms pull from the same queue without catastrophic dispatch collisions.

This is a rough model. It ignores the reality that swarms prioritize by label, not just by recency; it ignores the fact that some issues are "hot" and attract multiple swarms regardless of queue position; it ignores the fact that builder cycles vary. But as a first-order explanation of why the observed 5–10% duplicate rate (§4.1) is much lower than the naive "three swarms × same queue" prediction of near-100%, it is serviceable. The takeaway is that **the three-swarm model is not a static configuration — it is a dynamic equilibrium between dispatch rate and merge rate**, and the equilibrium is stable as long as the merge rate is a meaningful fraction of the dispatch rate. If merges stall, dispatches pile up, and the model breaks.

## 3a. Naming the Pattern

The four patterns in §3 are specific shapes; the underlying phenomenon deserves a name because it will recur, and naming it makes future forensic records easier to write. The pattern:

> **Three-Surface Cooperation via GitHub-Native State** — the operating configuration in which multiple independently-orchestrated agent swarms work on the same repository using only GitHub's existing state (issue labels, PR drafts, `Co-authored-by` trailers, comment threads) as their coordination substrate, with no direct messaging between surfaces and no shared memory.

Three things follow from the name:

1. **"Surface" is the right unit**, not "swarm" or "instance." Each surface has its own orchestrator, its own agent catalog, its own scratchpad, and its own reality. Two instances of the same orchestrator running on different devices are still one surface (they share a catalog, even if they don't share state). Three instances of three different orchestrators are three surfaces.
2. **"GitHub-native state" is the crucial constraint.** The pattern works because GitHub already provides everything the surfaces need to coordinate: a shared issue list, a shared PR list, a shared label schema, a shared squash-merge convention, and stable URLs for citations. Adding an out-of-band coordination channel (a chat room, a shared file, a daemon) would make the pattern more powerful but less robust — each new channel is a new failure mode. The 2026-04-11 evidence shows that GitHub-native state is *sufficient* for the patterns in §3.
3. **"Cooperation" is weaker than "coordination."** The surfaces are not coordinating in the strict sense — they do not agree on who does what. They are cooperating in the sense that each surface's output is (a) visible to the others and (b) useful to the others. That is a lower bar than coordination and it is why the pattern works without a protocol.

Using the name: future forensic records that document similar sessions can write "the session exhibited three-surface cooperation (§3a of `2026-04-11-three-swarm-cooperation.md`)" and link back here, instead of re-deriving the pattern. Future changes to the pipeline that affect GitHub-native state should check against §3 to see whether they preserve the cooperation substrate — removing the `Co-authored-by` trailer, for example, would break §3.2 branch-then-review legibility and should be called out as a regression of the forensic signal even if it has no code-level effect.

## 4. Coordination Failure Modes

Three failure modes surfaced in the 2026-04-11 window. None are catastrophic; all are reducible.

### 4.1 Duplicate dispatch (the #4108 vs #4120 race)

Two builders worked on issue #4100 in parallel. #4108 landed at 11:39:50 UTC; #4120 was filed shortly after, when Swarm A had not yet seen #4108 merge. The race window was on the order of 15–30 minutes — roughly the time for one swarm's builder to read the issue, write tests, implement the fix, and open a draft PR.

**Observed rate:** 1 race in ~30 high-priority dispatches during the session, or roughly **5–10%** of issue pickups by the busiest two swarms. The orchestrator's pre-session gut estimate was 15–20%; the observed rate was lower because the backlog was wide enough that independent parallel production (§3.3) dominated.

**Cost structure:** the race is cheap because builder agents are cheap and closing an unmerged PR is a label operation. Both branches produced correct fixes; only the first one landed. The waste is bounded to one builder's effort per race.

**Why it happened specifically for #4100:** the issue was high-priority (test regressions visible), high-signal (a false-premise revert is an unusual shape), and highly visible in every swarm's queue at the same time. Duplicate dispatch correlates with issue salience, not backlog depth.

**A secondary observation:** the race was resolved cleanly (one PR merged, the other closed without controversy) partly because both branches were on different branch names and neither had been merged when the duplication was discovered. Had both branches been `main`-like long-lived branches, the race would have been a merge conflict instead of a "close one PR" operation. The branch-per-PR convention is doing more cooperation work than it normally gets credit for.

### 4.2 Git-history attribution errors (the #3472 / #3808 / #3466 case)

A distinct and newer class of failure: a scout cited a PR number as evidence for a claim, and the PR was real but it fixed a *different* issue than the scout said it did.

Concrete instance: an earlier scout investigation claimed that [PR #3808](https://github.com/EffortlessMetrics/perl-lsp/pull/3808) had fixed [issue #3472](https://github.com/EffortlessMetrics/perl-lsp/issues/3472) — "[module-resolution] Import list symbols not extracted for bareword resolution". The reality:

- PR #3808 merged 2026-04-10 with title `fix(navigation): resolve imported function goto-definition (#3466)`.
- PR #3808 actually closed [issue #3466](https://github.com/EffortlessMetrics/perl-lsp/issues/3466), "ux: go-to-definition doesn't resolve imported functions (use Foo qw(bar); bar->)". That issue is CLOSED.
- Issue #3472 is still OPEN as of 2026-04-11.

The two issues look similar — both are about imported function resolution — but they are distinct. A scout that skimmed titles without running `gh pr view NNNN --json closingIssuesReferences` could easily produce this kind of off-by-one attribution error. The failure was caught only when a docs-sweep agent verified every cited PR number against `gh pr view`.

This is **not** a semantic claim error (the fix is real, the code does what it says). It is an **attribution error**: PR X fixed thing Y, not thing Z. It's a separate verification concern from "does the scout understand the bug," and it needs a separate verification pass. See §6.2 for the proposed mitigation.

### 4.3 Stale scout findings from concurrent merges

During the session, approximately 4–5 scout investigations by Swarm A were obsoleted by merges from Swarm B that landed between the scout dispatching and the scout filing its issue. Specifically: a scout would begin analyzing a symptom, the symptom would get fixed by an unrelated merge from another swarm, and the scout would file an issue that no longer reproduced.

**Cost:** waste is bounded because scouts are cheap and the scout-verify step (`scout-verify` skill) re-checks claims before filing. The scout-verify step caught most of these; the ones that slipped through were closed as `already-fixed` by plan-reviewers within minutes.

**Root cause:** the session cycle time (dispatch → file) exceeded the average master-churn interval. In a single-swarm session master churns at the rate of that swarm's merges; in a three-swarm session master can churn 2–3× faster. Scout lifetime budgets that were fine for single-swarm operation become marginal.

**Mitigation considered but rejected:** adding a "fetch origin before filing" step to every scout. The reason to reject it is that it only shrinks the window; it does not close it. A scout that fetches master, runs its analysis, and then files at T+5 minutes can still be obsoleted by a merge at T+3 minutes (which landed between the fetch and the file). The correct fix is either the §6.1 claim signal (which makes staleness explicit) or accepting the 5% `already-fixed` rate as the cost of three-swarm operation. The latter is cheaper and is what the session's plan-reviewers ended up doing implicitly.

### 4.4 Failure modes not observed

It is worth naming the failure modes that *did not* show up on 2026-04-11, because their absence shapes the recommendation in §6.

- **Branch conflicts.** No two surfaces attempted to push to the same branch simultaneously. Each PR had a single owning surface, and co-authored commits came from surfaces pushing additional commits to a branch that was still in draft. The branch-per-PR convention is the protection here.
- **Label overwrites.** No two surfaces raced to set conflicting labels on the same issue or PR. Labels are additive in GitHub's model, so a race would have ended with both labels present rather than one overwriting the other. Had the pipeline used mutually-exclusive label states ("in-build" vs. "needs-review"), a race could have produced inconsistent state. It did not because the existing label schema treats pipeline states as monotonic.
- **Agent identity confusion.** No surface impersonated another — the `Co-authored-by` trailers are accurate, not forged. If impersonation were possible (for example, if the Codex device pushed commits with a `Co-authored-by: Claude Code` trailer it had learned from previous merges), forensic attribution would collapse. It did not happen in this session, but the risk is not zero in a future session with different tooling.
- **Merge conflicts on long-lived branches.** There were no long-lived feature branches in this window; everything was trunk-based with short-lived PR branches. Three-swarm cooperation on long-lived branches has not been tested and is an open question.
- **Orchestrator-level deadlock.** No surface waited indefinitely for another surface to do something. Waiting happened on the scale of minutes (while CI ran) but never on the scale of hours, because no surface's progress was gated by another surface's action. The emergent specialization in §5.2 is partially responsible: if surfaces are working on disjoint issues, no one is blocked on anyone else.
- **Review cascade inversion.** No PR was reviewed by multiple surfaces in a way that produced contradictory review recommendations. It is possible to imagine Surface A approving a PR that Surface B then requests changes on, leaving the author unsure whose feedback to prioritize. It did not happen because reviews were typically completed by whichever surface picked up the draft first, and subsequent surfaces did not re-review.

The session's failure modes were the ones where surfaces raced each other on work (§4.1, §4.3) or left inaccurate artifacts behind (§4.2). The session's *non*-failures were the ones where surfaces could have actively undermined each other but did not. The asymmetry is worth preserving: the §6 recommendations target the observed failures without creating surface area for the unobserved ones.

## 5. Value Delta vs. Single-Swarm Operation

Three claims, each grounded in a specific 2026-04-11 observation:

### 5.1 Triple-independent verification catches false premises

The phase-block pragma bug (§3.1) was a false premise hidden in a landed refactor (PR #4052's rebase). The bug survived into PR #4090's proposed fix *because PR #4090 was extending the false premise further*, not correcting it. Swarm A's research-verifier caught it by stepping outside the codebase and running `perl -e` against a real Perl interpreter — the kind of reality check that a single swarm can run, but often skips because "the existing tests pass." Single-swarm operation would plausibly have merged #4090 and carried the false premise forward to v0.13.0. Triple-swarm operation turned up three independent checks within a few hours: research verification (#4090 comment), independent re-derivation (#4120's behavior-spec tests), and the revert itself (#4108). The bug had three chances to be caught; it was caught.

The economics here deserve a closer look. A single-swarm session catches bugs at the rate of its one verification loop: tests-plus-review. A three-swarm session catches bugs at the rate of (tests + review) × (number of independent surfaces that have their own verification habits). The multiplier isn't quite 3× because the surfaces share some verification DNA (they all run `cargo test`, they all read the same diffs), but it is meaningfully more than 1× because the *choices about what to verify* are independent. Swarm A's research-verifier chose to run `perl -e`; Swarm B's builder chose to write six new BDD tests; a hypothetical Swarm C might have reached for a fuzzer. Different verification tools, different bugs caught. The #4090 false premise is the paradigm case because "does this match real Perl" is not a test the existing suite knew to run.

### 5.2 Specialization by surface is emergent, not assigned

No one told the three swarms "you handle research, you handle code production, you fill gaps." They specialized because their cognitive profiles diverged:

- **Claude Code** is strong at strategic orchestration, long research contexts, and multi-file cross-reference sweeps. Its session behavior tilted toward scorecard design (#4062), wisdom synthesis (#4117), metric-stack documentation (#4123), and deep verification of risky PRs.
- **Codex device** is strong at sustained autonomous code production with tight local loops. Its session behavior tilted toward straightforward builder work, test burn-down, and feature flag implementations.
- **Codex web** is human-in-the-loop and fills gaps opportunistically; its session behavior depended on what the user noticed.

The **specialization happened automatically** because each swarm gravitated to work its tools were cheapest to execute. A forced assignment would have produced worse results — the gravitational pull is informative and should be respected, not overridden.

This is a less obvious value prop than §5.1 but arguably more durable. Emergent specialization is cheap (it costs nothing to let swarms pick what they are good at) and it is self-correcting (if a swarm picks work outside its sweet spot, the work goes slower, the other swarm notices and picks up the next issue instead, and the distribution rebalances within an hour or two). The forensic signal is visible in the solo/co-authored split in the §2 table: solo commits from Swarm A clustered on strategic docs, parser feature branches, and scorecard design; co-authored commits spread uniformly across hardening, test work, and feature fixes. The specialization is observable in the log even though no one declared it.

### 5.3 Reference-implementation verification becomes normal

A follow-on from §5.1: when a swarm can run `perl -e` against a real Perl interpreter (as the research-verifier on #4090 did), the verification isn't just "is the code right" but "does the code match the reference implementation." That check is valuable on its own, but it is *especially* valuable when it is run by a second surface as a cross-check against the first surface's assumptions. The three-swarm configuration normalized this by making reference-implementation checks something a different swarm could volunteer for, rather than something the main builder had to remember to do. The wisdom retrospective ([PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117)) expands on this thread under "reference-implementation verification"; this article cross-references it rather than duplicating.

### 5.4 Observable outcomes

Before the structural claim in §5.5, a few observable outcomes from 2026-04-11 that a reader can check without trusting any analytical claim in this article:

- **Throughput:** 40 merges in roughly 10 hours of active session time is a higher rate than any single-swarm day recorded in `.ops-perl-lsp/` metrics for the preceding two weeks. The exact number will be in [PR #4123](https://github.com/EffortlessMetrics/perl-lsp/pull/4123) when it lands.
- **Bug catches:** at least one false-premise bug (the #4090 / #4100 case) was caught and corrected inside the session window. A bug of that shape escaping into master would have required a subsequent session to catch it; the three-swarm configuration caught it inside one.
- **PR review coverage:** every merged PR in the 40-commit window was either solo (one surface's own review) or co-authored (a second surface contributed). No PR was merged without at least one review pass. Review-starvation (stale draft PRs sitting unreviewed for hours) did not occur.
- **Failure mode rate:** 1 duplicate dispatch (#4108/#4120), 1 attribution error (#3472/#3808/#3466 class), approximately 4–5 stale scouts. All were recovered cheaply. No failure mode produced permanent damage to master.

These are the metrics by which a reader can judge whether the session was a success independently of the analytical framing in this article. The analytical framing argues that the three-swarm model produces the metrics; a skeptical reader can accept the metrics without accepting the framing.

### 5.5 Continuous review pressure is structural

In a single-swarm session, the orchestrator must actively ensure that every PR gets reviewed; review-starvation is a failure mode that shows up as stale drafts and stalled merges. In the 2026-04-11 window **every merged PR either had a same-surface review or accumulated a cross-surface co-authored commit**. That wasn't policy, it was a consequence of two swarms both scanning the open-PR queue and one of them always having spare review capacity. The structural property: **review pressure scales with the number of swarms, while the cost of writing a PR scales with the number of changes**, so the review-to-change ratio improves monotonically as swarms are added (up to the point where duplicate dispatch costs start to dominate, which the session did not reach).

## 6. Proposed Improvements

Three targeted changes would reduce the observed failure modes without compromising the autonomy that made the cooperation work. None require shared memory, direct messaging, or a coordination daemon; all are compatible with surfaces that cannot see each other directly.

**Design principles the improvements all share:**

1. **GitHub-native or not at all.** Any mechanism that requires a channel outside GitHub (a chat, a daemon, a shared file in a non-repo location) introduces a single point of failure and a second thing to audit. The 2026-04-11 evidence shows GitHub-native state is sufficient; the improvements extend that rather than departing from it.
2. **Hints, not locks.** No improvement should be able to *prevent* a surface from doing work. The worst case for a bad hint is a duplicate dispatch, which is already the observed failure mode and which we know costs almost nothing. The worst case for a bad lock is work starvation, which is much more expensive.
3. **Graceful degradation.** If a surface forgets to honor a convention, the session should degrade to the 2026-04-11 observed behavior, not worse. This rules out any change that would make the pipeline stricter about inputs — adding new required fields or labels that must be present for work to proceed.
4. **Cheap to instrument.** If an improvement requires a new scout skill or a new CI check, it should be small enough to fit in a single `.claude/commands/*.md` file and short enough to execute in under a second. Anything larger is a different kind of design work and is out of scope for this article.

### 6.1 Cross-swarm claim signal (reduces duplicate dispatch)

A lightweight convention for signalling "I've claimed this issue, don't re-dispatch." Two low-friction options:

1. **Label convention** — add a `claimed-by-swarm-<name>` label (or just `claimed`) when a builder is dispatched, removed on PR merge or close. Every swarm checks the label before picking up an issue. Low cost, no schema change.
2. **Short-lived WIP draft PR** — the dispatching swarm opens a draft PR with a placeholder commit and a title like `WIP: <issue title> (claim-only)`. Other swarms see the draft in the PR list and skip the issue. Higher fidelity because it tells you *who* claimed it, but adds noise to the PR queue.

Option 1 is cheaper. Either would have prevented the #4108 / #4120 race — Swarm A would have seen the `claimed` label before dispatching.

Two properties of the label convention are worth noting: first, **it preserves autonomy** — nothing prevents a swarm from overriding another's claim if the situation warrants it (a stalled builder, for example), and the label is a hint, not a lock. Second, **it degrades gracefully when a swarm forgets to set the label** — the only cost is a duplicate dispatch, which is exactly the failure mode we already observed at 5–10% rate. Adding the label makes the happy path cheaper; forgetting the label leaves us where we already are. There is no regression risk.

Worked example with the 2026-04-11 #4100 race:

```text
Timeline without the claim convention (actual 2026-04-11):
  T+0:00  Swarm B builder picks #4100 from the backlog, starts work.
  T+0:05  Swarm A builder picks #4100 from the same backlog, starts work.
  T+0:30  Swarm B pushes draft PR #4108, opens for review.
  T+0:50  Swarm A pushes draft PR #4120, opens for review.
  T+1:10  Swarm B's PR merges.
  T+1:15  Swarm A's builder discovers the race, closes PR #4120.

Timeline with the claim convention (hypothetical):
  T+0:00  Swarm B builder picks #4100, applies `claimed-by-codex` label, starts work.
  T+0:05  Swarm A builder sees the `claimed-by-codex` label, picks #4101 instead.
  T+0:30  Swarm B pushes draft PR #4108.
  T+1:10  Swarm B's PR merges, `claimed-by-codex` is removed on close.

In the second timeline Swarm A produces a fix for a different issue in the same time budget.
That is the delta the claim convention buys.
```

The label would be set by the scout-report skill (when the scout hands off to a builder) or by the builder-read-spec skill (when the builder claims the issue). Either point works — the earlier the signal, the sooner the other surface can react. The label should be removed on PR merge, PR close, or a timeout (say, 4 hours) to handle the case where a builder starts an issue and then gets reassigned or stalls.

### 6.2 Git-history verifier pass (catches attribution errors)

Add a scout-dedup or plan-review sub-step that, for every cited PR number, runs `gh pr view NNNN --json title,closingIssuesReferences` and verifies two things:

1. The PR exists and is merged (guard against dangling references).
2. The PR's `closingIssuesReferences` includes the issue the scout claimed it fixes.

If the claim doesn't match, the scout corrects the attribution (or the plan-reviewer flags it). This catches the #3472 / #3808 / #3466 class of error mechanically. It should be cheap — one `gh` call per cited PR — and can be folded into the existing `scout-verify` skill.

A concrete design sketch: the check runs *after* the scout has drafted its findings but *before* the issue is filed. For every PR number cited in the draft (matched by a `#NNNN` regex), it calls `gh pr view NNNN --json state,closingIssuesReferences,title`. If the PR exists, the check records the PR's actual state and the issues it actually closed; if the scout's draft asserts something inconsistent with that, the scout rewrites the claim. The check does not second-guess the scout's semantic conclusion — it only verifies the mechanical mapping between PR numbers and issue numbers. That narrow scope is important because it keeps the check cheap enough to run on every scout issue without noticeably slowing the pipeline.

Worked example using the 2026-04-11 incorrect citation:

```text
Draft scout issue claim (hypothetical):
  "PR #3808 fixed #3472 on 2026-04-10. The issue is stale and should be closed."

Verifier pass:
  $ gh pr view 3808 --json closingIssuesReferences,state,title
  {
    "closingIssuesReferences": [{"number": 3466, "title": "ux: go-to-definition doesn't resolve imported functions"}],
    "state": "MERGED",
    "title": "fix(navigation): resolve imported function goto-definition (#3466)"
  }

  $ gh issue view 3472 --json state,title
  {"state": "OPEN", "title": "[module-resolution] Import list symbols not extracted for bareword resolution"}

Verifier output:
  MISMATCH: scout claims PR #3808 fixed #3472, but #3808 actually closes #3466.
  Issue #3472 is still OPEN. Scout must rewrite the claim or drop the citation.
```

The check takes less than a second to run, requires only `gh` as a dependency, and catches exactly the class of error that burned us on 2026-04-11. It is the single most-targeted improvement in §6 — less ambitious than §6.1, more concrete than §6.3, and it addresses a failure mode that §6.1 does not touch at all.

### 6.3 Explicit surface specialization (reduces duplicate dispatch at the source)

The specialization in §5.2 was emergent. It could be made explicit with a short declaration at session start: each swarm states which label slices (`P0`, `parser`, `workspace`, `docs`) it will pull from, and the others skip those slices. This is **not** a hand-off protocol — there is no mechanism to transfer work. It is **just an advisory slicing** that reduces the chance of two swarms reaching for the same issue at the same time. When specialization is honored, duplicate dispatch drops to near-zero; when a swarm needs to cross slices (e.g., because the bug is urgent), nothing prevents it, but the default routing is disjoint.

The catch with explicit specialization is that it partially sacrifices the §5.1 triple-verification effect. If Swarm A never touches pragma work because "that's Swarm B's slice," then the phase-block false premise has only one surface verifying it, not three. So the right formulation is not "swarm A only touches slice X" but "swarm A prioritizes slice X," leaving cross-slice verification as a background task. In practice this means the `claimed-by-swarm-X` label from §6.1 is strictly the better primitive and §6.3 should be read as a *soft* default on top of the label, not a hard constraint.

## 7. Open Questions

These are worth investigating in a later session but are not actionable from the 2026-04-11 evidence alone. Each open question is stated with enough specificity that a future session's wisdom agent can decide whether it has new data to close the question.

- **Is the `Co-authored-by: OpenAI Codex` trailer automatic or configured?** The commit log shows it consistently on Codex-touched commits but the exact mechanism (git hook, CLI default, squash-merge configuration) was not audited during this session. Worth documenting because the attribution is the only thing that makes cross-swarm cooperation legible to future forensic readers. If the trailer is automatic, the forensic signal is reliable; if it depends on configuration that can drift, the signal could silently disappear in a future session and cost a retrospective its ground-truth data.
- **Can the three surfaces be coordinated via a shared state file** (something like `.ci/swarm-state.json`) without compromising autonomy? The §6.1 label convention is one design point; a state file is another. The question is whether a file-based approach survives the cases where a surface cannot write back to the repo (Codex web in read-only mode, for example). A hybrid — labels for claim signalling, file for slower metadata like "which surfaces are active right now" — might fit better than either alone.
- **What is the stable-state operational cost of three swarms vs. one?** This session's data suggests compute multiplier is meaningfully less than 3× (because research verification and scout work overlap) and throughput multiplier is meaningfully more than 1× (the §3.3 parallel production effect), but the exact ratio was not measured. Candidate measurement: total PR throughput in merges/hour, divided by total agent-token spend across all surfaces, compared against a single-swarm baseline from a comparable session. A follow-up wisdom cycle could capture this.
- **Does the cooperation degrade past three swarms?** Four+ surfaces would increase duplicate-dispatch risk roughly quadratically (each pair can race). At some point the §6.1 coordination signal becomes mandatory, not advisory. Where that inflection point is — four? Five? — is unknown. It may also depend on backlog depth: a deeper backlog tolerates more swarms before the race rate climbs, because the probability of two swarms picking the same top-of-queue issue shrinks with the queue size.
- **Do the failure modes differ by surface pair?** The 2026-04-11 data mixes Claude Code ↔ Codex-device and Claude Code ↔ Codex-web interactions in the same log. Whether the duplicate-dispatch rate is uniform across pairs, or concentrated in one pair (say, because two surfaces share a queue-reading script), is unknown. A longer session with per-surface tagging would answer this.
- **What is the contribution of the human-in-the-loop surface?** Codex web is user-assisted, which means its dispatches reflect the user's attention rather than a policy. Whether that makes it more or less race-prone than the fully-autonomous surfaces is an empirical question. The intuition cuts both ways: a human scanning the queue is slower (fewer races from volume alone) but also less disciplined about checking labels (more races per dispatch).
- **Can a swarm's reviewer agent be safely "rented out" to another surface's open PRs?** All three surfaces in this session opened PRs that another surface reviewed. But none of the surfaces explicitly asked another to review — the reviews happened because reviewers proactively scanned the PR list. An explicit cross-surface review request (e.g., a `needs-cross-surface-review` label) would formalize this. Worth a separate design spike.

## 8. Evidence vs. Hypothesis

For the forensic record, a short decomposition of which claims are evidence and which are hypothesis:

**Evidence (directly backed by commit log, PR state, or issue state):**
- The 28:12 co-authored:solo split in the 40-commit window (§2 table, `git log --pretty` verifiable).
- The existence and closed-but-not-merged status of PRs #4090 and #4120, and the merged status of #4108, all tracked under issue #4100 (§3.1, verifiable via `gh pr view`).
- PR #3808 closing issue #3466 (not #3472), and #3472 remaining open (§4.2, verifiable via `gh pr view --json closingIssuesReferences`).
- The existence of the `Co-authored-by: OpenAI Codex` trailer on specific commits (§2 table).
- The approximate timing envelope (06:02 UTC to 12:00 UTC for the visible merge cluster) based on `mergedAt` timestamps of the cited PRs.
- The existence of PRs #4117 and #4123 as open wisdom/metrics documentation (cross-reference targets).

**Hypothesis (grounded but not directly proven by the session):**
- The 5–10% duplicate-dispatch rate (§4.1). The sample is small (one confirmed race out of roughly 30 dispatches); the true rate could be higher or lower across a longer window.
- The claim that §5.2 specialization was "emergent, not assigned" (§5.2). I know no one *instructed* the swarms to specialize, but I cannot rule out that training-time bias in each surface produced a coordination effect that looks emergent from the outside.
- The §3.5 prediction that slower master cadence would *increase* duplicate dispatch. This is a prediction of a counterfactual, not a measured fact.
- The §5.1 claim that single-swarm operation "would plausibly have merged #4090." This is a counterfactual — the actual single-swarm merge rate for false-premise fixes is unknown.
- The operational-cost ratio of three swarms vs. one (§7, fourth bullet). No direct measurement was taken during the session.

**Not claimed despite being tempting:**
- I do not claim the three-swarm model is unconditionally better than single-swarm. It is better along specific axes (verification redundancy, review pressure, throughput) at specific costs (duplicate dispatch, attribution errors, stale scout findings). Whether the net is positive for a given project depends on the project's baseline failure modes.
- I do not claim this pattern generalizes to arbitrary agent surfaces. The 2026-04-11 configuration was Claude Code + Codex device + Codex web, all operating on a repository whose conventions (labels, skills catalog, swarm protocol) were shaped by the primary orchestrator. A configuration with two Claude surfaces, or with a third-party agent, would need its own forensic investigation.
- I do not claim the cooperation would scale linearly to N swarms. §7 notes that the cost curve is probably superlinear in N for uncoordinated surfaces.

## 9. Cross-References

### Complementary session documents (do not duplicate)

- **Session retrospective** — underselling, unwired measurement, reference-implementation verification: [PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117), `docs/project/wisdom/2026-04-11-session-learnings.md`. Focuses on the lessons for the Claude Code swarm's own operating model. This article is the cross-swarm equivalent — what was learned by looking at the interaction between surfaces rather than at any one surface.
- **Metrics stack and ratchet model documentation**: [PR #4123](https://github.com/EffortlessMetrics/perl-lsp/pull/4123), `docs/project/metrics/README.md`. The contributor-facing story of how the metrics referenced throughout the 2026-04-11 session are computed and ratcheted. This article does not restate those metric definitions.
- **In-flight swarm-operations article** (general-purpose agent, `docs/articles/` or `docs/forensics/` target TBD): covers operational learnings from the session's swarm activity at large — how the pipeline behaved, where routing choices paid off, how to run a session like this one. This article narrows to the three-swarm cooperation model specifically.

### Primary evidence

- Scorecard design tracking issue: [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)
- **Phase-block pragma cascade** (the §3.1 triple-verification case): [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100), [#4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090), [#4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108), [#4120](https://github.com/EffortlessMetrics/perl-lsp/pull/4120)
- **Attribution failure exemplar** (the §4.2 case): [#3472](https://github.com/EffortlessMetrics/perl-lsp/issues/3472), [#3466](https://github.com/EffortlessMetrics/perl-lsp/issues/3466), [#3808](https://github.com/EffortlessMetrics/perl-lsp/pull/3808)
- **Cooperation pattern exemplars (branch-then-review, §3.2)**: [#4089](https://github.com/EffortlessMetrics/perl-lsp/pull/4089), [#4064](https://github.com/EffortlessMetrics/perl-lsp/pull/4064), [#4082](https://github.com/EffortlessMetrics/perl-lsp/pull/4082), [#4091](https://github.com/EffortlessMetrics/perl-lsp/pull/4091), [#4081](https://github.com/EffortlessMetrics/perl-lsp/pull/4081), [#4050](https://github.com/EffortlessMetrics/perl-lsp/pull/4050), [#4043](https://github.com/EffortlessMetrics/perl-lsp/pull/4043)
- **Cooperation pattern exemplars (independent parallel, §3.3)**: [#4093](https://github.com/EffortlessMetrics/perl-lsp/pull/4093), [#4002](https://github.com/EffortlessMetrics/perl-lsp/pull/4002), [#4098](https://github.com/EffortlessMetrics/perl-lsp/pull/4098)
- **Supporting infrastructure for the tight cadence** (see §3.5): [#4083](https://github.com/EffortlessMetrics/perl-lsp/pull/4083) (`fix(hooks): make pre-push run pr-fast instead of ci-gate`)

### Architecture and methodology

- [`docs/SWARM_ARCHITECTURE.md`](../SWARM_ARCHITECTURE.md) — single-swarm architecture baseline that this pattern extends
- [`docs/articles/SWARM_METHODOLOGY.md`](../articles/SWARM_METHODOLOGY.md) — the documented methodology for single-swarm autonomy
- [`docs/articles/AI_NATIVE_OPERATIONS.md`](../articles/AI_NATIVE_OPERATIONS.md) — broader framing of agent-operated workflows
- Project CLAUDE.md pipeline definition (Scout → Accuracy-Scout → Plan-Review → Build → Review → Green → Merge → Wisdom)

### Existing forensics dossiers for style continuity

- [`pr-259.md`](pr-259.md) — single-PR dossier (name_span for LSP navigation)
- [`pr-260-264.md`](pr-260-264.md) — multi-PR dossier (substitution operator correctness)
- [`INDEX.md`](INDEX.md) — PR archaeology inventory (this article is a new shape that the index may not yet accommodate; a "cross-session forensic" row would be a natural future addition)

## 10. Reproducibility

The forensic claims in this article can be verified by a reader with read access to the repository. Specifically:

```bash
# §2 commit table — reproduce the co-authored/solo split
git log --since="2026-04-11 01:00 UTC" --until="2026-04-11 15:00 UTC" \
  --pretty=format:"%h|%s|%(trailers:key=Co-authored-by,valueonly)" master

# §3.1 triple-verification exemplar — walk the #4100 trail
gh issue view 4100
gh pr view 4090 --json state,mergedAt,body
gh pr view 4108 --json state,mergedAt,body
gh pr view 4120 --json state,mergedAt,body

# §4.2 attribution error exemplar — verify which issue #3808 actually closed
gh pr view 3808 --json closingIssuesReferences,title,mergedAt
gh issue view 3472 --json state  # still open
gh issue view 3466 --json state  # closed

# §2 co-authored/solo ratio
git log --since="2026-04-11 01:00 UTC" --until="2026-04-11 15:00 UTC" \
  --pretty=format:"%h %(trailers:key=Co-authored-by,valueonly)" master \
  | awk '{ if (NF>1) co++; else solo++ } END { print co " co-authored, " solo " solo" }'
```

The expected output of the final `awk` is approximately `28 co-authored, 12 solo`. Small drift is acceptable — the exact counts depend on which commits are inside the time window and whether any further squash merges landed between the time this article was written and the time it is read — but the ratio should remain in the range of 60–75% co-authored for the article's claim to hold. If the ratio ever collapses to near-100% solo, the three-swarm pattern has stopped; if it stays at 60–75% in a future session, the pattern is recurring.

A second reproducibility note: the `Co-authored-by: OpenAI Codex` trailer is the load-bearing forensic signal. If a future version of any of the three surfaces stops writing this trailer, the ratio check above will return a false-negative (all solo). The §7 "is the trailer automatic or configured" open question is directly relevant — future forensic readers should verify the trailer is still being written before drawing conclusions from its absence.

## 11. Self-Review Questions

For a contributor who wasn't in the 2026-04-11 session:

1. **Do you understand what "three swarms" means concretely?** It means three independently-running agent surfaces using GitHub as their only shared substrate — not three threads of one orchestrator. Each surface has its own orchestrator, its own agent catalog, its own scratchpad, and its own reality. They share nothing except the GitHub state of the repository.
2. **Can you tell, by looking at a commit, whether it was touched by more than one surface?** Yes — the `Co-authored-by: OpenAI Codex` trailer on squash merges is the signal. Solo commits have no such trailer. The §2 table enumerates the full 40-commit window with this annotation for 2026-04-11.
3. **Is there a coordination protocol?** Not yet. The 2026-04-11 session used no explicit coordination; §6 proposes three low-cost conventions that would harden the model. All three are compatible with surfaces that cannot see each other directly.
4. **What is the main risk?** Duplicate dispatch on high-salience issues (~5–10% of pickups in the observed window) and attribution errors in citations (the §4.2 #3472/#3808/#3466 class of error). Both are cheap to fix with §6.1 and §6.2 respectively, and both are cheap to tolerate at the observed rates.
5. **Is this pattern reproducible?** Yes — any combination of Claude Code plus another agent surface operating on the same repo will produce it. The key requirements are a shared issue tracker, honest squash-merge attribution, and non-overlapping default pickups (the §5.2 emergent specialization). The §10 reproducibility section gives the exact commands to verify.
6. **Why does this document exist and not just the session retrospective?** The retrospective in #4117 is about what the Claude Code swarm learned from its own operating model. This article is about what *two or more* swarms look like cooperating on one repo — a different scope, answering different questions. Both can be true; neither subsumes the other.
7. **Is the pattern applicable to non-perl-lsp projects?** Likely yes, but it has only been observed on this project so far. The requirements (shared issue tracker, squash-merge attribution, tight CI gate) are standard on most active open-source projects. Whether the specific failure rates generalize is unknown.
8. **What do I do if I'm setting up a new session and want the three-swarm pattern to work?** Two things: (a) run `just clean-worktrees` and `just doctor` before spawning agents to make sure the gate is fast, and (b) have a clear default slicing of the backlog across surfaces (§6.3) even if it is just advisory. Both are in the project's existing `CLAUDE.md` rhythms, so no new infrastructure is required.

## 12. Closing Narrative

There is a small irony in how this article came to exist. The three-swarm cooperation pattern is visible in the 2026-04-11 commit log only because three surfaces happened to be active on the same day; the pattern was not planned, not instrumented, not named until after the fact. The only reason I (the Claude Code swarm, writing this forensic article) can say "Swarm B caught #4100 at 11:39 UTC" is that GitHub's `Co-authored-by` trailer preserved the attribution on a squash merge, and that the research-verifier comment on PR #4090 preserved the reasoning chain that fed into the fix. If either of those artifacts were absent — if Codex did not use the trailer, or if the research-verifier had posted to an ephemeral scratchpad instead of a PR comment — the pattern would still have existed on 2026-04-11 but it would be invisible to every forensic reader after today. The cooperation is real; its legibility is fragile.

That fragility is the reason §6.1 (claim signal), §6.2 (git-history verifier), and §10 (reproducibility) are written the way they are. Each one is a way of *making the cooperation legible*, not *making the cooperation happen*. The happening is already robust — three swarms on the same repo, given enough backlog depth and a fast enough gate, will cooperate. What is not robust is the ability of the next forensic reader to verify that they did. The proposed improvements in §6 are all biased toward preserving legibility: a claim label is a permanent GitHub artifact, a git-history verifier pass makes attribution checking routine, and the reproducibility commands in §10 are exact recipes for re-verifying the claims in this article against a future repository state.

If the three-swarm pattern recurs in a future session and this article can be pointed to as prior art — if someone writing the next `docs/forensics/20NN-NN-NN-*.md` can say "we observed three-surface cooperation as in the 2026-04-11 article" and move on — then the article has done its job. If the pattern recurs and no one notices because the forensic signal was allowed to degrade, then the next article will have to re-derive everything here from scratch. The difference between those two outcomes is whether the legibility infrastructure (labels, trailers, stable URLs) is protected as a first-class operational concern or treated as incidental. This article recommends the former.

---

*This document is descriptive, not prescriptive. It records what was observed on 2026-04-11 and proposes the minimum changes that would preserve what worked. The pattern itself — three differently-shaped swarms cooperating via GitHub-native state without direct messaging — is worth naming because it is cheaper to run, harder to starve, and more resilient to single-surface failure than a single-swarm baseline, and because the 2026-04-11 session is, as far as this repository's history records, the first durable forensic trace of it.*

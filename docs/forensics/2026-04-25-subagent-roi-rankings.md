# 2026-04-25 — Sub-Agent ROI Rankings

**Lens**: Which agent types delivered the most value per token spent during the 2026-04-25 high-volume session
**Purpose**: Reusable agent-selection guide for future operators

Across ~80 sub-agent dispatches in the session, distinct agent types showed wildly different value-per-token profiles. This doc ranks them by observed ROI with concrete examples.

---

## The rankings (highest to lowest ROI)

### Tier S — Highest ROI

**Direct orchestrator API ops** (no sub-agent at all)
- Cost: ~16 gh CLI calls in ~4 minutes wall-clock + zero sub-agent tokens
- Output: 6 label applications + 1 strip + 2 ci-green + 3 needs-ci-fix + 3 closures + 1 promotion + 1 admin-merge = 17 mutations
- Why it wins: when the action is mechanical and the rationale is predetermined, a sub-agent adds tokens for no judgment value
- When to use: end-of-session label catchup, queued action items from earlier waves, anything where the operator already knows what to do

**reviewer-deep (sonnet)**
- Cost: ~30-40 min runtime per agent, ~80-150k tokens
- Output this session: caught 7 distinct correctness bugs (#6088 keyword regex, #6379 array literal, #6053 dead code, #5403 fixture matrix, #5361 lexer interpolation break, #5368 off-by-one, #6230 always-failing CI gate); pushed 4+ fix-forward commits
- ROI multiplier: each bug catch prevents hours-to-days of post-merge debugging; verification ladder memory entries say 12-16× cumulative ROI
- When to use: any PR with semantic risk (parser, lexer, workspace, security, incremental cache); any PR that earlier agents marked uncertain

### Tier A — High ROI

**ensemble-curator (general-purpose with cluster-triage prompt)**
- Cost: ~20-30 min runtime per agent, varies with cluster size
- Output this session: closed 108+ duplicate PRs across 9 subsystem clusters in single afternoon
- ROI multiplier: highest closure throughput per token; one agent handles 5-15 PRs at once
- When to use: any time Codex bursts have stacked 3+ PRs against the same issue or scope
- Caveats: **must include "DO NOT close where uncertain" instruction** (load-bearing — see #5989/#5990 wrong-closure recovery); **must include "skip already-covered: #N1, #N2" lists** to avoid collisions

**diff-auditor (haiku)**
- Cost: ~15-20 min runtime per agent, ~40-80k tokens
- Output this session: identified .hermes/ contamination across 8 PRs, caught .snap.new artifacts, validated coherence across ~50 PRs
- ROI: high — catches artifact-class issues that downstream gates miss
- Caveats: false positives on stale-base interpretations (saw maintainer-pr-L confusion); need to specify ".spec/<wave>/ folders are project history, NOT scope drift"

### Tier B — Medium ROI

**standards reviewer (haiku)**
- Cost: ~15 min runtime, ~30-60k tokens
- Output: applied review-reviewed labels to ~25 PRs, caught bare assert! violations in #6379/#6203/#6239/#5938
- ROI: solid for label-application + banned-pattern checks; limited judgment depth
- When to use: post-diff-audit standards gate

**maintainer-pr (haiku)**
- Cost: ~15-20 min runtime, ~40-80k tokens
- Output: applied maintainer-pr-reviewed labels to ~30 PRs across batches A-L
- ROI: variable — most batches were sound but **batch L produced systematically wrong "branch contamination" verdict on 8 PRs** (false positive caught by verifier-of-verifier pass)
- Caveats: cheap-model agents reading metadata fields like `--changedFiles` can drift; instruct to use REST `additions`/`deletions` for diff size questions

**green-CI (haiku)**
- Cost: ~20-30 min runtime, ~50-100k tokens
- Output: classified ~60 PRs across batches A-K into GREEN/STALE-RED/REAL-RED/MERGE-CONFLICT
- ROI: medium — necessary work but **had recurring misclassification of per-PR fmt failures as master cascade** (5/12 false positive rate per master scout)
- Caveats: instruct to verify master health BEFORE declaring cascade; instruct to use first-line-of-failure matching, not just check-name matching

### Tier C — Variable ROI (high when right scope, low otherwise)

**pr-responder (haiku)**
- Cost: ~20-40 min runtime depending on task
- Output: variable by task — bulk update-branch (good throughput), fmt-fix wave (7/12 hit rate), needs-ci-fix classification (mixed)
- ROI: scales with task tractability; struggles when conflicts need real judgment (e.g., the 17 hard-conflict PRs all returned BLOCKED)
- When to use: clearly-scoped 1-line fixes, label correction with predetermined rule, bulk update-branch on stale-base PRs
- Avoid: hard merge conflicts requiring judgment (those are builder work, not responder work)

**ops (sonnet)**
- Cost: ~10-15 min per drain pass, ~30-60k tokens
- Output: 10 actual merges across 4 drain passes (waves 1, 3, 5, plus direct-CLI finishing)
- ROI: high per merge but each pass merges only 2-4 PRs (CI cancellation cascade prevents larger batches)
- When to use: after each promotion sweep returns; **iterate, don't one-shot**

### Tier D — Low ROI in this session (situational)

**Memory consolidation review (general-purpose)**
- Cost: dispatched but ran out of quota mid-task
- Output: incomplete; produced no consolidation proposal
- Caveat: the task is genuinely useful (memory has reached useful density per the meta-learnings doc) but needs to be re-run with dedicated quota allocation
- Verdict: don't blame the agent; the task got Anthropic-quota-exhausted, not because the work is low-value

**Worktree cleanup analysis (general-purpose)**
- Cost: ~15 min, ~30k tokens
- Output: 1 worktree removed (from 174), conservative posture left others alone
- ROI: low — disk overhead doesn't matter day-to-day; cleanup is more useful as part of a session-start ritual than a session-end concern

**Aged-PR triage (general-purpose)**
- Cost: ~5 min, ~30k tokens
- Output: zero (no PRs older than 10 days in the queue)
- ROI: zero this session because the aging window was wrong; would have been higher with a 3-day or 7-day window

---

## Selection heuristics derived from rankings

When choosing which agent to dispatch:

1. **Can the orchestrator do it directly?** (mechanical action, predetermined rationale) → don't dispatch a sub-agent
2. **Does it need correctness judgment?** (parser, lexer, security, semantic) → reviewer-deep, not standards/maintainer
3. **Are there 3+ PRs with same issue/scope?** → ensemble-curator with explicit skip-list
4. **Is it bulk classification with clear rules?** → diff-auditor or standards (haiku tier)
5. **Is the task "investigate why CI fails"?** → split: green-CI for classification, pr-responder for narrow fixes, builder for hard cases
6. **Is it cluster-fixing 5+ PRs that all share root cause?** → don't dispatch per-PR; file tracking issue (e.g., #6715), fix upstream, cascade-update

---

## Anti-patterns observed

- **Dispatching maintainer-pr without policy specifics** (e.g., the .hermes attribution policy wasn't specified, agent executed default = strip-all = wrong) — every recurring policy area should be in the prompt explicitly
- **Dispatching ensemble agents on overlapping PR sets without skip-lists** — produces wrong-closures (caught for #5989/#5990; harmless but wasteful)
- **Dispatching reviewer-deep on already-closed PRs** — agent reviews and labels a CLOSED PR (#6090). Add "check PR state at start" preamble
- **Dispatching ops drain as a one-shot** — produces 0-2 merges before CI cascade triggers; iterate in batches of 3 with 30s gaps
- **Dispatching green-CI without "verify master before declaring cascade" instruction** — produces recurring false-master-cascade signals

---

## Tokens-per-merge / tokens-per-closure (rough)

Order of magnitude estimates from the session:
- Direct orchestrator API op: ~0 sub-agent tokens per action (10-100 prompt tokens for the orchestrator turn)
- Ensemble closure (haiku): ~5k-10k tokens per PR closed
- ops merge (sonnet): ~20k-40k tokens per PR merged (includes the full review chain leading up to it)
- reviewer-deep bug catch: ~80k-150k tokens per bug caught
- Cluster triage: ~3k-8k tokens per PR processed (large amortization across cluster)

**Implication**: closures are 4-10× cheaper per unit than merges, and direct API ops are essentially free. Optimize the wave shape for closure throughput first, then merges, with sub-agents reserved for judgment-required work.

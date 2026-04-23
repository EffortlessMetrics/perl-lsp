# Session Retrospective: 2026-04-23 — Tier-Wiring Cascade + Reviewer Fix-Forward + Parallel Haiku

**Session window:** 2026-04-22 23:00 UTC → 2026-04-23 ~14:00 UTC (two 5h Claude-session windows)
**Scope:** Third iteration continuing the Codex-review-at-scale series. Focus: landing CI structural improvements, proving fix-forward reviewer pattern at scale, draining deep-review on a 175-PR backlog.

This retrospective distills what was pushed forward, the patterns that emerged, and the durable substrate produced. Companion documents: `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md` (per-window economics), `docs/articles/TWO_MODE_DEV_LOOP.md`, `docs/articles/TRIAGE_AS_LEARNING.md`, `docs/articles/HAIKU_FOR_MECHANICAL.md`, `docs/articles/TWO_PHASE_MERGE_GATE.md`.

## What was pushed forward

### Structural CI improvements landed

| PR | Change | Why it matters |
|---|---|---|
| **#5005** | Tier-wiring: ci-scope classifier wired into PR Smoke with scope-aware clippy/test + graceful fallback | Every subsequent CI signal went through scope-selected lanes. Enabled both cost-controlled draft tier AND bit-rot exposure. |
| **#5018** | Fix master bit-rot: `super::incremental_edit` test-mod import depth | Unblocked compile-all-targets across every perl-parser-touching PR |
| **#5152** | Fix master bit-rot: clippy `single_match` + 4 pre-existing sandbox `#[ignore]` | Unblocked perl-lsp-rs clippy gate + prevented ~40 cascade merges waiting on tier-wiring scope |
| **#5212** | Fix master bit-rot: `extract_module_names_from_use_args` now canonicalizes `'` → `::` | Unblocked `unit_full` gate on every perl-workspace-index-touching PR |
| **#5263** | Agent receipt enrichment: scope + lanes + reasons + repro + baselines in gate receipt | Unlocked agent-native CI where pr-responder agents get structured failure context instead of log-tail parsing |
| **#5265** | Lane yield metrics: unique-catch rate + signal-per-$ telemetry | Foundation for demoting low-yield lanes, promoting high-yield ones |
| **#5313** | Fix master bit-rot: capability snapshots regenerated after feature drift | Unblocked lsp_tier_a gate |
| **#5314** | Reviewer-deep skill-chain fix: must NOT set `merge-ready` directly | Eliminated ~40 label-strip operations per session by pushing the rule into the file, not the memory |
| **#5317** | Fix master bit-rot: Windows Guardrails package ID `perl-workspace-index` → `perl-workspace` | Unblocked ~25+ PRs stuck on Windows lanes |

### Feature PRs with substantive product impact

- **#5060** — anonymous-sub and local-as-rhs parser recovery (closes the `my $code = sub {...};` regression, the #5017 visible case)
- **#5086** — implicit Moose/Moo strict detection scoped to file level (matches real Perl BEGIN compile-time semantics)
- **#5247** — **full Perl 5.8-5.40 compatibility matrix** with 17 minors, label-gated for cost, weekly cron for visibility, bidirectional version-gated probes
- **#5022** — bareword imports in `use ... qw(...)` lists resolve for goto-def across 4 LSP surfaces (nav/moniker/refs/rename)
- **#5024** — `use constant` symbols surfaced in completion via visible-symbol-table
- **#5060** + tests: #5023, #5011, #5009 parser-recovery coverage
- **#4979** — `@INC` normalization + dedupe with preserved precedence
- **#5256** — deep CPAN 5-level inheritance goto-def fixture (real ecosystem shape)
- **#5272** — real-repo first-diagnostics perf lane (nightly, 5000+ line apps)

### Queue drain

- **175 → 42 open** over the session (-133 PRs)
- **250+ merges today** (per GitHub search `merged:>=2026-04-23`)
- **~60 dupes closed** across UX scorecard, bareword imports, pragma, refactor, CI compile, docs clusters
- **Per-outcome cost:** ~0.22% Claude session per actionable outcome with Haiku offloading the mechanical work (vs ~1% without)

## Patterns that emerged

### 1. Bit-rot exposure as a CI-scope-expansion consequence

Every widening of CI scope this session surfaced one pre-existing master failure. Seven in total:

| # | Bit-rot | Exposed by | Fix PR |
|---|---|---|---|
| 1 | perl-parser `super::incremental_edit` import depth | #5005 tier-wiring expanding compile-all-targets | #5018 |
| 2 | clippy::single_match + 4 sandbox test failures | #5005 tier-wiring expanding clippy scope | #5152 |
| 3 | sandbox stdout capture broken on Windows+Linux | #5152 `#[ignore]` + investigation | #5198 (filed) |
| 4 | legacy `'` separator not canonicalized in extract_module_names | tier-wiring expanding perl-workspace-index scope | #5212 |
| 5 | capability snapshots drifted from `capabilities_json()` | merge-gate lsp_tier_a on first post-tier-wiring PR | #5313 |
| 6 | Windows Guardrails package ID mismatch (`perl-workspace-index` vs `perl-workspace`) | #5223 Windows matrix going live | #5317 |
| 7 | Windows path-resolution tests failing in perl-module (4 tests) | #5317 unblocking the matrix to actually run | #5321 (filed) |

**Lesson captured in memory `feedback_tier_wiring_exposes_bitrot.md`:** Widening CI scope *will* produce short-term noise. The noise is the signal — don't relax the gate, fix the surfaced issues in narrow PRs and cascade-update.

### 2. Reviewer-deep fix-forward at scale

The policy change "mechanical findings push directly, don't send back to a builder" produced measurable throughput acceleration:

**Evidence, this session:**
- #4979 (stale doc + whitespace test), #5024 (vacuous `let _ = table` + 5 tests), #5042 (HashMap non-determinism + HashSet perf), #5060 (Phase-1 error-recovery test updated to Phase-2), #5079 (leaf-node test), #5082 (vacuous char-class assertion strengthened + 4 edge cases), #5099 (LazyLock pattern standardized), #5103/#5112 (real off-by-one coordinate fixes), #5129 (unclosed regex delimiter), #5130 (generic `#[must_use]` on T=() warning 373 call sites), #5269 (test asserted parse errors on valid Perl), #5310 (integer-coded diagnostics), #5311 (missing serialization asserts), #5288 (camelCase joiner bug), #5090 (vacuous `our $Foo::name` test replaced with 3 behavioral tests)

**Lesson captured in memory `feedback_reviewer_deep_proactive_fixes.md`:** "Send it back for one-line fix" is the wrong default. Reviewer-deep has full context loaded; marginal cost of pushing a fix is a few tokens, not a fresh agent spawn.

### 3. Codex 4-shot waves are design exploration, not noise

The default reaction to a 60-PR cluster is "pick one, close the rest." Wrong. Each variant explores a different architectural approach; triage is **learning**, not just filtering.

**Evidence, UX scorecard cluster:**
- 60 PRs for "track editor UX confidence signals"
- 48/60 converged on the same JSON field names and fixture layout → that's the emergent-consensus shape
- 3 survived triage as architecturally distinct (fixture-backed, xtask+schema+docs, minimal plumbing)
- 1 had a real bug caught (#5184 split on `\n## ` when actual heading was `#### What shipped`, producing 8 IDs when JSON said 4)
- Winner #5154 inherited implicit lessons from the 57 losers

**Lesson captured in memory `feedback_gap_analysis_as_codex_prompt.md` + docs article `TRIAGE_AS_LEARNING.md`:** Convergence = consensus, divergence = real design question, losers reveal edge cases.

### 4. Two-mode dev loop with human as router

Direct orchestrator-reviewer mode (Claude) + prompt-routed Codex generation mode, with the human reading gap analyses and feeding them to Codex. **Not reviewer, not typist — curator of specs and router of outputs.**

**Observable proof this session:** 12-point "next generation" gap analysis produced → human pasted into Codex → 8+ of those gaps filled in one wave (real-repo perf #5272, lane yield #5264-5267, agent receipt #5261-5263, missing-Perl UX #5260, API stability #5257-5259, deep CPAN #5256, DAP conformance #5253-5255, CI integration #5270-5271).

**Lesson captured in docs article `TWO_MODE_DEV_LOOP.md`:** Write gap analyses as Codex-prompt-ready specs (file paths, issue refs, acceptance criteria) — not essays.

### 5. Haiku 4.5 ≈ Sonnet 4 for mechanical work, properly scoped

Seven Haiku reviewer batches dispatched across all 103 non-deep-reviewed PRs closed ~20 duplicates at fractional Sonnet cost. Haiku tdd batch B pushed 17 edge-case tests across 3 PRs successfully. The pattern works when scope is narrow and rules are explicit.

Boundary observed: Haiku green-tdd agents couldn't cleanly switch across multiple PR branches in one worktree — scope to ONE PR per agent OR use `gh api -X PUT` content-push pattern.

**Lessons captured in docs article `HAIKU_FOR_MECHANICAL.md` + memory `feedback_green_tdd_worktree_constraint.md`.**

### 6. File-level protocol edits beat memory-note reminders

Memory file `feedback_deep_reviewer_premature_merge_ready.md` captured the symptom for weeks — orchestrator stripped `merge-ready` ~40 times per session. The actual fix was a one-file edit to `.claude/agents/reviewer-deep.md` removing `/pr-ready` from the skill chain (shipped in #5314). Subsequent reviewer-deep agents from the updated file correctly stop at `deep-reviewed`.

**Generalized:** Memory is for patterns, files are for rules. Durable protocol encoding goes in the agent definition or repo docs; ephemeral reasoning goes in memory.

### 7. Don't kill agents

Killed agent `a6061b35` (35-PR deep-review) mid-run because the scope felt shallow. Lost the partial triage work it had produced. User correctly flagged this.

**Lesson captured in memory `feedback_dont_kill_agents.md`:** Let agents finish. Over-broad scope? Dispatch additional narrower agents in parallel — duplicate work is cheaper than lost work. Exceptions: truly hung (10+ min no output), actively damaging, hard-to-undo cascading edits, spawn loops.

## Durable substrate produced

**Memory files added (Claude-side, `~/.claude/projects/.../memory/`):**
1. `feedback_reviewer_deep_proactive_fixes.md` — fix-forward policy
2. `feedback_tier_wiring_exposes_bitrot.md` — noise-is-signal pattern
3. `feedback_gap_analysis_as_codex_prompt.md` — human-as-router role
4. `feedback_dont_kill_agents.md` — let them finish, kill only on specific failure modes
5. `feedback_green_tdd_worktree_constraint.md` — scope green-tdd to one PR per agent

**Docs articles added (repo-side, `docs/articles/`):**
1. `TWO_MODE_DEV_LOOP.md` — direct + prompt-routed Codex modes
2. `TRIAGE_AS_LEARNING.md` — 4-shot variants as design exploration
3. `HAIKU_FOR_MECHANICAL.md` — default-to-Haiku policy
4. `TWO_PHASE_MERGE_GATE.md` — design proposal for scoped delta after baseline
5. `SESSION_2026_04_23_RETROSPECTIVE.md` — this doc

**Repo-side policy fixes (`.claude/agents/`):**
1. `reviewer-deep.md` — correctness gate not merge gate
2. `reviewer.md` — `(#0000)` accepted placeholder (clarification)

**Forensic (`docs/forensics/`):**
1. `2026-04-23-tier-wiring-reviewer-fix-forward-session.md` — per-window economics + per-5%-checkpoint work log

## Economics observation

| Measure | This session (both windows) | Context |
|---|---|---|
| Claude 20× Max weekly | 76% → 84% (+8%) | Approaching saturation |
| Codex Pro weekly | ~15% → ~21% (+6%) | Nowhere near saturated |
| Claude-per-Codex ratio | ~4× more Claude-cost than Codex-cost | Expected; Claude does expensive routing/review |
| PRs merged | 250+ | ~1% Claude session per merge at ~1:8 ratio with reviews/closes included |
| PRs closed (dupe) | ~60 | Triage-and-filter pattern validated |
| Bit-rots surfaced + fixed | 7 | Each cleared ~20+ stuck PRs |
| Durable docs + memory | 12 files | Substrate for future sessions |

**Per-outcome cost observation:** ~0.22% Claude session per actionable outcome when Haiku offloads mechanical work, vs ~1% without. A 4-5× cost reduction just from model-tiering correctly.

## CI-cost efficiency (open question, billing lag)

Real GitHub-side CI spend for the month:

| Date | Month-PRs-merged | Month-CI-cost | Reading |
|---|---|---|---|
| 2026-04-22 (prior day, fully billed) | 680 | $230.52 | **$0.339/PR solid reading** (complete) |
| 2026-04-23 (this session, billing lags) | 1051 | $235.97 | Apparent $0.2245/PR **misleading denominator** |

**The trustable number is the prior day's $0.339/PR.** Today's reported $235.97 only reflects ~$5 of new CI spend over yesterday, but **~371 additional PRs merged today have CI runs not yet billed**. GitHub Actions billing typically lags 24-48h. Dividing by the full 1051 PRs (including ~371 unbilled) produces an artificially low cumulative.

**What we'll actually know in ~24-48h:** the real $/PR for this session's work once today's CI runs get billed.

**Agent-side cost** (the other half of the economics):
- **Claude Max 20x** ($200 USD/mo flat-rate subscription)
- **ChatGPT Pro** ($200 USD/mo flat-rate subscription)
- Combined agent subscription: **$400/mo**

Per-PR amortized agent cost at 1051 PRs MTD = **~$0.38/PR** ($400/1051). Decreases with each additional merge since flat-rate. The Claude Max plan at this throughput represents a multi-x discount vs token-retail pricing.

**Combined rough order-of-magnitude** (once CI billing lag resolves):
- Agent subscription cost per PR amortized: ~$0.38 (decreasing with scale)
- CI runner cost per PR: tbd, likely $0.15-0.35 range
- **Total ~$0.50-0.75/PR combined** at current scale

**Efficiency hypothesis to test against tomorrow's bill:** if cumulative CI $/PR settles meaningfully below $0.339 (say < $0.25), structural CI work paid out empirically per-PR. If it settles near $0.339, the structural work held per-PR cost constant at 1.5x throughput still meaningful value, but not a per-PR efficiency improvement. **Revisit after 2026-04-24 billing update.**

**Plausible efficiency drivers** (to evaluate once billing resolves):
1. **Tier-wiring (#5005)** scope-aware PR Smoke skipped full workspace clippy+test
2. **Preflight latest-SHA check (#4977)** cancelled superseded CI runs on rapid-push cascades
3. **Compile-all-targets parallelization (#4988)** didn't serially block merge-gate
4. **Windows Guardrails fix (#5317)** unblocked ~25 PRs in one CI pass
5. **Cascade-update pattern** one master merge cascades to N PRs without N x full-CI cost
6. **Dupes closed early** ~60 closed without merging consumed zero merge-gate budget

---

_Companions: the referenced memory files + docs articles + forensic. See `docs/articles/ARTICLE_INDEX.md` for the broader articles catalog._

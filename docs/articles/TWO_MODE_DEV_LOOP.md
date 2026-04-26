# Two-Mode Dev Loop: Direct + Prompt-Routed

**Date:** 2026-04-23
**Session:** 3rd iteration continuing the Codex-review series

Working observation from this session and the surrounding 2026-04-22 → 2026-04-23 window: real-production-quality agentic dev on perl-lsp runs in **two complementary modes**, and they feed each other.

## Mode A — Direct orchestrator-reviewer

Claude orchestrates, spawns reviewer-deep / builder / research-verifier / reviewer / ops agents in parallel, pulls gh status, merges when ready, closes duplicates, files gap issues. Full loop lives inside Claude's context window with the human available for high-level routing.

**What this mode is good for:**
- Substantive code review that needs full repo context
- Surfacing bit-rot as scope widens (#5005 tier-wiring → #5016/#5152/#5198/#5212/#5313)
- Fix-forward on correctness bugs (coordinate-space in #4999, clippy in #5152, canonicalize in #5212)
- Gap analysis (produces the prompting material for Mode B)
- Merge-queue drain coordination across cascading dependencies

**Economics:** roughly 1% Claude session per actionable outcome at 20× Max pricing.

## Mode B — Prompt-routed Codex generation

Human reads Claude's gap analysis, feeds it to Codex Web as a prompt. Codex produces **4 PRs per prompt** (its design, not a bug) attacking the same gap from different architectural angles. Claude then triage-learns-improves: closes dupes, extracts patterns, picks winner, deep-reviews, merges.

**What this mode is good for:**
- Mass application of a policy (e.g., 60 PRs adding "editor UX confidence signals")
- Rapid coverage of a defined API surface (e.g., all 14 DAP handlers)
- Producing enough variants that the emergent consensus is visible
- Cheap synthesis when the task is well-bounded by a spec

**Key insight — each variant is a different attempt with different ideas, not noise.** Codex's 4-shot-per-prompt produces architecturally distinct approaches — one may expose a field in `symbol.rs`, another may route through `scope_analyzer.rs`, a third may live entirely in `completion.rs`. These aren't random variations; each is a genuine design exploration. Triage is **learning, not filtering**:

- **Convergence across variants** = emergent consensus on the approach (e.g., 48 of 60 UX-scorecard PRs emitted the same JSON field names → that shape is probably right).
- **Divergence across variants** = the real design question ("fixture-backed vs live-collection vs markdown-parsing" surfaced as the genuine architectural split in the UX scorecard cluster; the 3 surviving winners each embodied one approach).
- **Patterns the "losers" used** often reveal edge cases the winner missed (#5269 caught the winner's "unclosed expression" test because a "losing" variant had a different broken-Perl fixture that failed to parse while the "winner" used syntactically-valid Perl by mistake).

The "losers" are training data on approach space. When I deep-review the winner, I implicitly inherit the exploration that produced it.

**Economics:** trades Claude-review spend for Codex-generation spend. Codex Pro weekly has been nowhere near saturated through 200+ PR sessions.

## How the two modes feed each other

```
Direct mode                    Prompt-routed mode
-----------                    ------------------
1. Do the work                 1. Codex produces 4×N variants
   (review / merge / fix)         against a spec

2. Surface gaps while         2. Claude triage-learns-improves
   working                        (close dups, extract patterns,
                                   pick winner, deep-review)
3. Write gap analysis         3. Merged PRs feed
   as Codex-ready spec            gap analyses for next cycle
   (file paths, issue refs,
   acceptance criteria)
                ↓                              ↓
         spec becomes             improvements feed
         Codex prompt             direct-mode decisions
```

The human is the router between modes — reads Claude's gap synthesis, pastes into Codex, drops the wave back at Claude. Not a reviewer; not a typist. **A curator of specs and a router of outputs.**

## Concrete patterns that emerged this session

### Bit-rot exposure pattern

Tier-wiring (scope-aware CI scope) landed in #5005. Subsequent waves exposed 5 pre-existing master failures:

| Issue | Cause | Fix |
|---|---|---|
| #5016 | Wrong `super` depth in `incremental_document.rs` test mod | #5018 one-line |
| #5152 | Pre-existing clippy::single_match on perl-lsp-rs | #5152 `match` → `if let` + 4 sandbox `#[ignore]` |
| #5198 | Sandbox stdout capture broken (empty output treated as success) | filed for later |
| #5212 | `extract_module_names_from_use_args` returned `Foo'Bar` not canonical `Foo::Bar` | #5212 applied `canonicalize_perl_module_name` |
| #5313 | Capability snapshots drifted from `capabilities_json()` output | #5313 `UPDATE_SNAPSHOTS=1` |

Each would have leaked into v0.13.0 without the scope expansion. The noise wasn't over-strictness; it was accumulated debt surfacing.

### Fix-forward reviewer pattern

Mechanical findings go directly on the PR branch, not back to a builder. Commit message + PR comment document the finding AND the fix. Clean diff, one logical change, clear intent.

Evidence this session: #4979 (stale doc comment + whitespace-only test), #5024 (vacuous `let _ = table` → real assertion + 5 tests), #5042 (non-deterministic HashMap error order), #5060 (stale Phase 1 error-recovery test updated), #5079 (missing leaf-node test), #5082 (vacuous char-class assertion strengthened), #5099 (non-standard `LazyLock<Result<Regex>>` → repo standard), #5103/#5112 (off-by-one coordinate fixes), #5129 (unclosed regex delimiter + pre-existing clippy), #5130 (generic `#[must_use]` on T=() warned across 373 call sites), #5269 (test asserted parser errors for syntactically valid Perl).

10× cheaper than spawning a builder, because the reviewer has full context already loaded.

### Reviewer-deep skill-chain fix (2026-04-23)

`.claude/agents/reviewer-deep.md` had an instruction to run `/pr-ready` (which sets `merge-ready`) after approving. This bypassed the `green-ci` + `diff-audited` receipts in the state machine. The orchestrator stripped `merge-ready` ~40 times this session. Fix: removed the `/pr-ready` invocation from the reviewer-deep todo list and updated the "final quality gate" framing. **Correctness gate, not merge gate.** Memory was insufficient — the FILE had to change.

### Gap-analysis-as-prompt pattern

Produce a structured "what's missing" list with file paths, issue numbers, acceptance criteria. Human routes it to Codex. Codex returns a targeted wave. Observed 2026-04-23: I produced a 12-item gap analysis; within ~20 minutes Codex delivered PRs addressing 8+ items (#5272 real-repo perf, #5264-5267 lane yield, #5261-5263 agent receipt, #5260 missing-perl UX, #5257-5259 API stability, #5256 deep CPAN, #5253-5255 DAP conformance, #5270-5271 CI integration).

## Corrections applied to the 12-point "next generation" analysis

The original analysis (see session forensic 2026-04-23) listed 12 improvements. After user review:

| # | Original framing | Correction |
|---|---|---|
| 1 | "Codex back-pressure: one PR per spec" | **Wrong.** Codex's 4-shot-per-prompt is a design feature. Triage+learn+improve pattern is the right shape. |
| 2 | Agent-native CI self-propelling | Useful, models considered, takes work. Defer. |
| 3 | 30-second merge-gate | Two-phase model: full gate on first approval caches baseline; subsequent pushes only run scoped delta. |
| 4 | Fuzz for incremental parse | Yes, improve. |
| 5 | Perf baselines with teeth | Yes, improve. |
| 6 | Reviewer-deep skill-chain fix | **Done this session** — file edit, not memory note. |
| 7 | Cross-tool parity | Yes, improve. |
| 8 | Release engineering | **v0.13.0rc1** is the next release tag — not final 0.13.0 yet. |
| 9 | Semantic conflict detection | Not needed — 4-shot waves are cheap. |
| 10 | Memory federation / AGENTS.yaml | **Bigger.** Documentation substrate needs to be heavy — CLAUDE.md, repo docs, AGENTS.md-level refs. Way more heavily than current. |
| 11 | Failure-mode-triggered policies | Haiku 4.5 handles mechanical work at close-to-sonnet-4 quality. Use Haiku more aggressively. |
| 12 | Orchestrator observability + interruption | Testing approaches out. Direct mode is one of the most productive methods; don't abandon it. |

## The direct-mode value that's easy to miss

"Chucking ideas at Codex and then improving and fixing and merging the improved PRs" produces real improvement **surprisingly efficiently when scoped right**. But this is not the whole loop — the direct work I'm doing right now (reviewing, fix-forwarding, closing dups, surfacing bit-rot, writing forensics) is one of the most productive direct repo-working methods available. Both modes matter. Mode B scales throughput; Mode A provides the substrate that makes Mode B's output worth merging.

---

_Companion documents: `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md`, `docs/articles/ORCHESTRATION_COUNTERINTUITIONS.md`, `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md`. Memory: `feedback_reviewer_deep_proactive_fixes.md`, `feedback_tier_wiring_exposes_bitrot.md`, `feedback_gap_analysis_as_codex_prompt.md`._

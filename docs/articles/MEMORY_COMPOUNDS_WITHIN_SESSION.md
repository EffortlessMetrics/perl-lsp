# Memory Compounds Within a Session, Not Just Across Them

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp (11 hours, ~60 agent spawns)
**Cross-references**: [KNOWLEDGE_COMPOUNDING.md](KNOWLEDGE_COMPOUNDING.md), [CLAUDE_MD_EVOLUTION.md](../project/CLAUDE_MD_EVOLUTION.md)

---

## TL;DR

The usual framing for AI memory systems is cross-session: "remember today's decisions so future sessions don't relearn them." The 2026-04-19 Wave G1 session produced a less-discussed use case — memory writes serve as **context-window extensions within a single long session**.

Over 11 hours and ~60 agent spawns, the orchestrator wrote a principle to memory at turn N ("don't rubber-stamp agent verdicts; take judgment") and cited that same memory file at turn N+30, when a similar situation arose. The agent's own context window couldn't have held the full reasoning 30 messages back — but the memory file, loaded into context on each turn, carried it forward.

Memory isn't just durable persistence. It's a form of working memory that survives context compression within the same session.

---

## The Concrete Case

**Turn N — scope-pivot insight emerges.** After reversing the first diaboli DEFER on #4497 (scope-pivoted from 74 crates to 5 facades, see [SCOPE_PIVOT_ON_DEFER.md](SCOPE_PIVOT_ON_DEFER.md)), the user challenged: "we're taking judgment of everything, not just accepting its findings." The orchestrator reflected on the pattern and wrote it to memory:

```
feedback_take_judgment_on_verdicts.md

Verification agents (diaboli, oppositional-planner, architecture-reviewer,
maintainer) produce *advice*, not directives. The orchestrator — me — owns
the decision. A DEFER verdict is a recommendation to weigh, not a block to
apply automatically.
```

The memory file was one paragraph plus concrete example references. At the time, the orchestrator expected this principle to be useful in the future — maybe the next day's session, maybe a week later.

**Turn N+30 — same principle, new context.** A second diaboli DEFER arrived on #4499 (offline manifest-lint). The orchestrator's immediate instinct — honed by the first reversal — was to re-examine scope. Memory confirmed: the take-judgment principle was the right framing. The orchestrator pivoted #4499's scope (6 checks → 2), reversed the DEFER, shipped the PR. Same session.

Between turn N and turn N+30: ~100,000 tokens of intervening conversation — verification pipeline on 4 other issues, PR review pipelines, 2 CI-flake incidents, rename decisions. The conversational context was long gone. But the memory file remained in the orchestrator's CLAUDE.md-loaded context on every turn.

## What Memory Does That Conversation Can't

A long session accumulates insights, but the insights in the conversation are *sequentially dependent*. Reference turn 4's insight at turn 60 requires the full turn 4-59 to be in context, which they rarely are at that distance.

Memory files break the sequential dependency. An insight written to memory is:
- Accessible in any subsequent turn.
- Independent of intervening conversation.
- Summarized — the write includes the distilled principle, not the full 20-turn conversation that produced it.

This is structurally similar to how humans use notebooks. Working through a problem, you jot down the insight as it crystallizes. Later, rereading the notebook gives you the insight without reconstructing the reasoning.

## When To Write Mid-Session

Not every turn should produce a memory write. Three triggers that worked well this session:

1. **User challenges a behavior and the reflection reveals a principle.** "We're taking judgment of everything" prompted reflection on what that principle actually was. Writing it out clarified it for the current turn and future reference.

2. **A pattern repeats within the session.** When scope-pivot worked on #4497 and later looked applicable to #4499, the repetition said "this is a pattern, not a one-off." Patterns earn memory writes.

3. **The insight is at risk of being lost to context compression.** If the insight lives in a long conversational chain that might be compressed or trimmed, and the insight is useful at later turns, it earns a write.

## When Not To

- **Ephemeral session state.** "The current PR is #4510" is task-list territory, not memory. Rewriting it means the memory would contradict the next session.
- **Codebase facts.** "The package name is `perl-lsp-rs`" is a `grep` away; memory duplicates.
- **Things already in CLAUDE.md.** Don't write to memory what's already in the session-loaded project context.
- **Corrections to prior memory.** Update the existing memory file; don't write a new one.

## What This Means For Memory System Design

Two implications for how a memory system should work:

1. **Memory reads should be cheap.** If memory is loaded into context on every turn (as in perl-lsp's CLAUDE.md + MEMORY.md pattern), the marginal cost of having a useful principle available is zero per turn. Writing a principle is a one-time cost; reading it is free. This economics favors more writes, not fewer.

2. **Memory organization should support scannability.** MEMORY.md's one-line-per-entry index means a 30-entry list is readable at a glance. The orchestrator doesn't need to load 30 full memory files; it reads the index and loads the relevant few. This is a design choice — it keeps the cost of "having lots of memory" low.

## Five New Memory Files From This Session

Sessions that produce patterns produce memory. The 2026-04-19 session produced eight memory files total, with five net-new ones written during (not after) the session:

- `feedback_take_judgment_on_verdicts.md` (turn N; used at turn N+30)
- `feedback_scope_pivot_on_defer.md` (turn N+40; refines the above)
- `feedback_ci_runs_lib_tests_only.md` (turn N+15; used at turn N+50 when a builder reported clean `--lib` tests but bit-rot remained in `--tests`)
- `feedback_nested_worktree_main_switch.md` (turn N+45; consulted at turn N+55 when firing another worktree-isolated agent)
- `feedback_reweigh_prior_comments.md` (turn N+70; written after an external AI advisory was noticed to be stale)

Each was written when the insight first crystallized. Each was read or referenced later in the same session.

## Related

- [KNOWLEDGE_COMPOUNDING.md](KNOWLEDGE_COMPOUNDING.md) — cross-session memory compounding (the complementary pattern)
- [CLAUDE_MD_EVOLUTION.md](../project/CLAUDE_MD_EVOLUTION.md) — how the project's memory format evolved
- [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) §8 — specific session data

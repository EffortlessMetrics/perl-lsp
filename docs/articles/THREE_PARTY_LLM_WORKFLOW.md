# The Three-Party LLM Workflow

What it's like running a development loop with a CTO, a research-partner LLM (longer architectural context), and an executor LLM (repo-local truth and CI ground truth) — and what makes it work, what breaks, and why the substrate-specific roles matter.

> Companion: [../reference/AGENT_HANDOFF_PROTOCOL.md](../reference/AGENT_HANDOFF_PROTOCOL.md) for the operating rules. This doc is the synthesis of *what we observed*, not the rules.

## What's actually unconventional

Running a project with one human and one LLM is now common. Running with one human and two LLMs in different *substrate-matched* roles is less common, and the role split matters.

The three parties in this loop:

| Party | Vantage | Latency | What they catch |
|---|---|---|---|
| CTO/product (human) | Strategic intent, public communication, risk tolerance | Slow (judgment) | Strategic drift |
| Research partner (e.g., ChatGPT with GitHub access) | Longer architectural memory; reads PRs/issues/diffs but not chat | Slow (transcript checkpoints, minutes-to-hours) | Architectural drift |
| Executor (e.g., Claude Code with repo access) | Repo-local truth, CI state, working tree, failing logs | Fast (seconds-to-minutes) | Ground-truth drift |

Plus GitHub as the durable substrate visible to all three.

## The asymmetry is a feature

The slow loop and the fast loop are not in tension. They're complementary because:

- **Strategy** lives in the slow loop. Architectural framing, scope decisions, sequencing, review criteria — all best-served by patience and longer memory. Rushing the slow loop loses the patience.
- **Tactics** live in the fast loop. CI state, log inspection, mechanical recovery, immediate next action. Batching the fast loop loses the responsiveness.

The dread case for asymmetric latency is when both layers need to react in real time and they compete for cycles. In this loop, they don't compete — strategy thinks while tactics execute, then tactics report back when ready for the next strategic input.

The latency mismatch *enforces* the right division of labor. If both layers ran at the same speed, we'd be tempted to use them interchangeably and lose the role distinction.

## Defense in depth via mismatched vantage points

Each layer has different blind spots. The system is robust because the blind spots don't overlap.

**CTO catches strategic drift:**
- "Don't bundle MSRV/toolchain into an install patch."
- "0.13.3 should be a focused install-reliability patch, not a feature release."
- "I'll handle the issue closeouts; don't touch them."

**Research partner catches architectural drift:**
- "One symptom may be two bugs — the Windows EBUSY could be source-side AND destination-side."
- "Versioned managed install dirs are the durable architectural fix; retry alone won't survive a running-binary lock."
- "The published-extension smoke needs to pass for both Marketplace and Open VSX before the release is proven."

**Executor catches ground-truth drift:**
- "Your packet quotes a GraphQL error, but the actual log says `pending CI state`. Different cause, different fix."
- "PR #7871 was auto-closed by GitHub when its base branch was deleted; we need a replacement, not a rebase."
- "v0.13.3 actually shipped — the orchestration was re-dispatched after the timing-race recovery. All channel smokes are green. Your packet's premise is stale."

A failure mode any one layer would miss, another catches. Not because anyone is always right — because the blind spots are non-overlapping.

## Orchestration floats by context-ownership

Authority is fixed (the CTO is always CTO). Operational orchestration moves to whoever has the freshest context for the current question:

- **Research partner leads** when the question is architecture, sequencing, or review criteria. ("Should this be one PR or two? What's the right test surface?")
- **Executor leads** when the question is repo state, CI state, or what just happened. ("The orchestration validation just failed; here's the actual log; here's what changed since your packet.")
- **CTO leads** when the question is scope, appetite, or public communication. ("Defer #7832. Don't touch issues. Land #7870 today.")

This is opposite to conventional engineering process where the orchestrator is a fixed role (a tech lead, a release manager, a product manager). Here the orchestration role floats by who has the freshest lens, which works only if every handoff explicitly re-asserts state.

## What the executor's load-bearing role actually is

The temptation is to treat the executor as a fast pair of hands — execution throughput. The actual load-bearing work was different.

Of the high-leverage moments during the v0.13.3 closeout, the throughput-shaped ones (write the code, run the tests, open the PR, merge) were maybe 40% of the value. The rest was **ground-truth callbacks**:

- "Your packet's premise has changed. The release already shipped."
- "The error you quoted isn't in the actual log. Here's what the log says."
- "The PR you want me to retarget was auto-closed."
- "The smoke test is masking the real failure mode — it's hitting destination-lock first, never reaching source-lock."

These are the moments where the slow loop is operating from a stale snapshot and the fast loop's job is to *push back firmly with evidence* before executing. They're the difference between a smooth recovery and a wasted iteration.

The protocol implication: the executor's first step on every cycle is **verify the premise**, not "execute the next action." If the premise has drifted, surface it; don't silently proceed.

## What works

**Scope packets.** Every packet from the research partner ends with explicit `Include` / `Exclude` / `Stop gates`. This is what prevents yak-shaving and keeps narrow PRs narrow. Without "do not touch issues," the executor would have drifted into closeout work and lost focus on the install rail.

**Failure taxonomy stability.** The executor classifies every failure as product / workflow / flake / state-drift / permission. Once classified, downstream layers trust the label without re-deriving. That trust loop only works because the labels mean the same thing across cycles. See [../reference/FAILURE_CLASSIFICATION.md](../reference/FAILURE_CLASSIFICATION.md).

**Decision tables when scope or blast-radius differs.** When the executor faces a choice between options with different risk profiles, the format is a table with `Option / Scope / Fixes / Risk / Recommendation` rows. This worked perfectly for the source-lock vs. versioned-dirs vs. both-A-and-B decision during the install-hardening work.

**Receipts as bridge.** When the executor is idle for 25 minutes between wakeups, the local receipt file is the persistent state that survives. It's also what the research partner can read when writing the next packet. Without it, both loops would re-derive state from scratch every cycle.

## What broke

**Premise drift was the dominant failure.** Three concrete cases this session:

1. Research-partner packet assumed a closed PR could be retargeted; GitHub had auto-closed it.
2. Research-partner packet quoted an error message that didn't appear in the actual log.
3. Research-partner packet treated a shipped release as unshipped because the snapshot it was written from predated the recovery.

Each case cost a partial wasted iteration. The fix is mechanical (Step 0: verify the premise), and it's now in the protocol, but at the time we hit it organically.

**Self-imposed friction.** The executor added a "pause to confirm before high-blast-radius action" that wasn't authorized — the packet had explicitly authorized release dispatch with the exact command. The pause cost a wakeup cycle. Now codified as: if authorized, proceed without re-asking; surface deltas, not confirmations.

**Walls of text in summaries.** The executor's progress reports were sometimes long where they should have been tight. The format that worked best was a status table + brief narrative + clear next-action call.

**Pre-emptive wakeup scheduling.** The executor scheduled wakeups for itself to check CI state. This was useful for not blocking, but it means the human doesn't know *when* the executor will be back. A better pattern is to schedule wakeups with explicit "I'll come back at <time> unless you interrupt earlier" so the human knows the cadence.

**Bundling drift.** A release-prep PR landed without the corresponding `RELEASE_HISTORY.md` row, breaking master CI for *all* downstream PRs. The bundling assumption (version bump and ledger update land together) was implicit in the orchestration but not enforced. Now codified as a known seam-fragility.

## Architecture is doing as much work as the substrate

The pace of this loop — five PRs landed in coherent chain over a single overnight session, including a public release — is downstream of the codebase architecture, not just the LLM substrate. Specifically:

- **Microcrate decomposition** means small fixes have small change vehicles. A 50-file PR for one bug would not have been tractable in this timeframe regardless of substrate.
- **Tests-as-spec** means the proof of a fix is encoded as code, not as a Slack thread. The next contributor inherits the proof.
- **Drift gates** (release-history, version-sync, install-surface) catch master-state issues immediately, even when humans miss them.
- **Atomic version surfaces** (one canonical `Cargo.toml` workspace version, propagated mechanically) make release-prep a single-PR operation.

If the codebase required broad-scope behavioral changes, the LLM substrate would be the wrong tool. The substrate works because the *change vehicle is small*. That's an architectural property, not a substrate property.

## Anti-patterns surfaced

Things that look efficient but break the protocol:

- **Compressing PR bodies for skim-readability.** PR bodies are the durable execution record; the audience is future LLMs (and engineers) reconstructing context. Optimize for density, not brevity.
- **Trusting research-partner verdicts over executor ground-truth.** The slow loop's premises decay; the fast loop's reports are fresh. When they conflict, ground-truth wins until proven otherwise.
- **Bypassing the handoff ledger because "the work was small."** Even small sessions accumulate state. If the next cycle has to re-derive it from transcript, the ledger should have existed.
- **Asking permission for actions already authorized.** Costs cycles and signals untrust. Surface deltas instead of confirmations.
- **Bundling unrelated work into release-prep PRs.** Drift gates expect ledger updates and version bumps to land together. Unbundling them creates master-state breakage that blocks downstream PRs.

## Implications for future loops

The maturity step that this loop is partway through:

```
Session 1 (this one): high-performing improvisation
Session N+1:          codified protocol with mechanical premise-verification
```

The protocol artifacts (this doc, the handoff doc, the failure-classification doc) are the codification step. They convert "we figured out what works" into "the next loop starts knowing what works."

The next loop's executor reading these docs as part of its onboarding is the test of whether the codification is complete. Things that need re-derivation aren't yet codified.

## Provenance

Pattern observed during the v0.13.3 install-reliability release closeout (2026-05-03). Five PRs landed (`#7870`, `#7872`, `#7876`, `#7877`, `#7874`) across one overnight session through the Steven (CTO) + ChatGPT (research) + Claude (executor) loop. Specific frictions catalogued in `docs/forensics/2026-05-03-*.md`.

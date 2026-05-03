# Agent Handoff Protocol

How the four-party LLM orchestration loop runs: human (CTO/product), research-partner LLM (longer architectural context), executor LLM (local repo + CI ground truth), and GitHub (durable shared state). This doc describes what makes the loop reliable and what each handoff must contain.

> Companion docs: [FAILURE_CLASSIFICATION.md](FAILURE_CLASSIFICATION.md), [RELEASE_PROOF_PROTOCOL.md](RELEASE_PROOF_PROTOCOL.md), [../articles/THREE_PARTY_LLM_WORKFLOW.md](../articles/THREE_PARTY_LLM_WORKFLOW.md).

## The four parties

| Party | Vantage point | Latency | Authority |
|---|---|---|---|
| **CTO/product** (human) | Strategic intent, risk tolerance, public communication | Slow (judgment cycles) | Direction-setting; final authority on scope and dispatch |
| **Research partner** (e.g., ChatGPT) | Longer architectural memory; reads GitHub state but not chat | Slow (transcript checkpoints, minutes-to-hours per cycle) | Planning, sequencing, review criteria, scope packets |
| **Executor** (e.g., Claude Code) | Repo-local truth, CI state, working tree, failing logs | Fast (seconds to minutes per cycle) | Implementation, recovery, ground-truth callbacks |
| **GitHub** | Durable cross-party state: PRs, issues, diffs, checks, releases, labels | N/A | Persistent record; the only state visible to all three |

GitHub is the only party with durable state across all three cycles. Anything that needs to survive a session without re-derivation must land in GitHub: PR bodies, issue bodies, commit messages, dated docs in `docs/`, or release artifacts.

## Why the loop works

Each layer covers a different failure surface:

- **CTO** catches strategic drift ("don't bundle MSRV/toolchain into an install patch")
- **Research partner** catches architectural drift ("retry alone won't handle a running-binary lock; need versioned dirs too")
- **Executor** catches ground-truth drift ("the failing log doesn't say what your premise quoted; the PR is already closed; v0.13.3 already shipped")

Failures one layer would miss, another catches. The system is robust not because any layer is right but because the blind spots are non-overlapping.

## Asymmetric latency is a feature, not a bug

The slow loop (research partner) is for strategy. The fast loop (executor) is for tactics. Trying to make the slow loop realtime loses architectural patience; trying to batch the fast loop loses responsiveness. The protocol explicitly preserves the asymmetry: research-partner packets arrive on slow cycles and stay valid for tens of minutes; executor cycles complete in seconds and return ground truth.

## Orchestration floats by context-ownership

Authority is fixed (CTO is always CTO). Operational orchestration moves to whoever has the freshest context for the current question:

- Research partner leads when the question is **architecture, sequencing, or review criteria**.
- Executor leads when the question is **repo state, CI state, or what just happened**.
- CTO leads when the question is **scope, appetite, or public communication**.

A handoff is the explicit transfer of orchestration to the layer best-positioned to answer next. Every packet should be implicit or explicit about which layer is leading the next turn.

## Step 0: verify the premise before acting

The dominant failure mode of this protocol is **premise drift** — the executor is given a packet that was true when written but stale by the time it arrives. The protocol's mechanical fix is that every executor cycle starts by re-asserting the premise:

```
Ground truth at <time>:
  origin/master SHA: ...
  current branch: ...
  open PRs: ...
  release status: ...
Delta from your packet: ...
Now executing: ...
```

When delta is non-empty, the executor must surface it to the CTO before continuing. This converts "I notice you're operating on stale data" from a stochastic judgment call into a verification step.

Examples where this rule would have prevented round-trips this loop has hit:

- Packet assumed `#7871` would auto-retarget after its base branch was deleted; actually GitHub auto-closes such PRs. Step 0 (`gh pr view 7871 --json state,baseRefName`) catches this immediately.
- Packet quoted "GraphQL: Could not resolve PullRequest with the number of 0" as the validate-release failure; actual log said "Commit ... is not in a successful CI state (pending)". Step 0 (`gh run view <id> --log-failed | grep error`) confirms which.
- Packet treated `v0.13.3` as unpublished; the executor had already recovered and the release was live with all smokes green. Step 0 (`gh release view v0.13.3`) confirms.

## Authorization is not re-confirmed

Once the CTO has authorized an action with explicit parameters, the executor proceeds without re-asking. Re-asking costs a wakeup cycle and signals lack of trust the CTO has already extended.

A "pause to confirm before high-blast-radius action" is appropriate **only** when:

1. The packet's premise has materially changed since authorization, OR
2. Executing the obvious next step would expand scope beyond what was authorized.

Both cases imply the executor should surface the change as a Step-0 delta, not as a confirmation question.

## Decision-table format

When the executor must choose between options before proceeding, the format is:

```markdown
Decision needed

| Option | Scope | Fixes | Risk | Recommendation |
|---|---|---|---|---|
| A | ... | ... | ... | ... |
| B | ... | ... | ... | ... |
| C | ... | ... | ... | ... |

Default: <option>
Proceed mechanically after decision?: yes/no
```

Trigger conditions for invoking the table (executor should know when to prompt vs. proceed):

1. Executing the obvious next step would expand scope beyond the packet.
2. The choices have meaningfully different blast-radius profiles (irreversible vs. reversible; broad vs. narrow).
3. The choice is hard to reverse without rework.

If none of the three apply, the executor proceeds with the obvious choice and reports.

## PR packet template

Used by research partner → executor for any PR-shaped work item:

```
PR PACKET

Title:                (e.g., fix(release): auto-record RELEASE_HISTORY row after publish)
Goal:                 (one-sentence user-visible invariant)
Scope include:        (file/area list)
Scope exclude:        (explicit non-goals — usually larger than include)
Implementation constraints: (things that must / must not happen)
Required tests:       (what proves the invariant)
Verification commands: (executor runs these locally before opening the PR)
Stop gates:           (when to halt and surface a decision)
Autonomy:             (open PR draft / open ready / merge if green / dispatch downstream)
Return shape:         (what the executor reports back; usually delta + PR URL + verification log)
```

The `Scope exclude` line is load-bearing. It is what prevents yak-shaving and stops the executor from drifting into unrelated work. Every packet should include it explicitly.

## Handoff ledger

When a session that crosses the executor/research-partner boundary closes, the executor writes a handoff ledger. This is the durable record of "what changed during this session" that survives transcript expiration.

Format:

```markdown
# Handoff: <date> — <topic>

Canonical state at session end:
- repo: ...
- origin/master SHA: ...
- relevant open PRs: ...
- relevant run IDs: ...
- active blockers: ...
- target release: ...

Merged this session:
- PR ...: <subject> — squashed at <SHA> — verification: <link>

Opened this session:
- PR ...: <subject> — state: <draft/ready/merged> — purpose: ...

Needs human decision:
- <item>: options + recommendation

Next mechanical action:
- <step the next executor or research partner should take>

Parked / out of scope:
- ...

Do-not-touch:
- issues: ...
- releases: ...
- workflows: ...
```

**Physical home**: committed at `docs/handoff/<date>-<topic>.md` as part of the closing PR's changes (or as a follow-up docs PR if the closing PR is narrow). This makes the ledger GitHub-visible (the research partner can read it on its next cycle) and survives any local-state loss.

Loose `target/receipts/*.md` files are evidence under the protocol but **not** durable handoff records — `target/` is gitignored. Don't conflate the two.

## What the protocol assumes

This protocol works because:

1. **The codebase is modular** — narrow PRs are possible; small fixes have small change vehicles.
2. **CI gates are strong** — review-by-test is a defensible substitute for human code review on narrow changes (it does not scale to broad behavioral changes).
3. **Failure taxonomy is stable** — labels mean the same thing across cycles; downstream agents can trust them.
4. **GitHub is the durable substrate** — anything not on GitHub effectively does not exist for the next cycle.
5. **CTO sets crisp scope boundaries** — without "do not touch X" / "include Y" / "exclude Z", the executor drifts.

When any of these break, the protocol degrades. The fix in each case is structural, not behavioral: harden the architecture, the gates, the taxonomy, or the durable surface — don't ask the executor to be more careful.

## Anti-patterns

Things that look efficient but break the protocol:

- **Compressing PR bodies for skim-readability.** PR bodies are the durable execution record; the audience is future LLMs (and engineers) reconstructing context. Optimize for density, not brevity.
- **Bypassing the handoff ledger because "the work was small."** Even small sessions accumulate state. If the next cycle has to re-derive it from transcript, the ledger should have existed.
- **Asking permission for actions already authorized.** Costs cycles and signals untrust. Surface deltas instead.
- **Trusting research-partner verdicts over executor ground-truth.** The slow loop's premises decay; the fast loop's reports are fresh. When they conflict, ground-truth wins until proven otherwise.
- **Bundling unrelated work into release-prep PRs.** Drift gates expect ledger updates and version bumps to land together. Unbundling them creates master-state breakage that blocks downstream PRs.

## Provenance

Pattern emerged during the v0.13.3 install-reliability release closeout (2026-05-03), with five PRs landed across one overnight session through a Steven (CTO) + ChatGPT (research) + Claude (executor) loop. Specific frictions are catalogued in `docs/forensics/2026-05-03-*.md`.

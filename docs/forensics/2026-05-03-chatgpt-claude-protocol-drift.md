# 2026-05-03 — ChatGPT/Claude Protocol Drift

**Lens**: How the asynchronous research-partner ↔ executor LLM loop drifts under premise staleness, and what fixes both sides should adopt.
**Outcome**: Three concrete drift instances during the v0.13.3 closeout, all recovered by the executor's ground-truth callbacks. Codified into the handoff protocol's "Step 0: verify the premise" rule.

## The drift pattern

The research partner (ChatGPT, in this loop) writes packets based on transcript checkpoints. By the time the executor (Claude Code) reads the packet, the executor has typically moved — fixes have landed, recoveries have completed, state has changed. Each packet has built-in latency between "premise true" and "packet executes."

When the executor silently follows a stale packet, the result is either a no-op or a re-introduction of a failure mode the original recovery already addressed.

## The three concrete cases

### Case 1: Auto-closed PR vs. assumed-retargetable

**Packet premise** (paraphrased): "After #7870 merges, GitHub will auto-retarget #7871 from the deleted hardening branch to master."

**Reality**: GitHub *closes* PRs whose base branch is deleted, rather than retargeting. The executor checked:

```bash
$ gh pr view 7871 --json baseRefName,state,mergeable
{"baseRefName":"fix/vscode-managed-install-013-3","mergeable":"CONFLICTING","state":"CLOSED"}
```

**Recovery**: opened a replacement PR (`#7872`) with master as the base, cherry-picked the release-prep commit forward.

**Cost**: ~2 minutes for the executor to diagnose + re-open. Would have been zero if the packet had said "verify #7871's state before retargeting; if closed, open a replacement."

### Case 2: Misquoted error message

**Packet premise** (paraphrased): "The release-orchestration validate-release step failed because it queries `PullRequest(number: 0)` via GraphQL — that's the seam to fix."

**Reality**: the actual `Validate Release` log says:

```
##[error]Commit 06fc1443dc... is not in a successful CI state (pending)
##[error]Process completed with exit code 1.
```

There is no GraphQL call in the failing step. The validation uses `gh api repos/.../commits/$SHA/status --jq '.state'`. The failure is a timing race against post-merge master CI, not a PR-number-0 lookup.

**Recovery**: the executor retrieved the actual log, surfaced the discrepancy, and proceeded with the correct diagnosis (timing race, recoverable by waiting for master CI to report).

**Cost**: one round trip — the executor pushed back with evidence rather than executing the wrong fix.

### Case 3: "Release didn't ship" packet vs. shipped release

**Packet premise** (paraphrased): "v0.13.3 didn't publish from run 25273613531; the validate-release gate needs fixing before re-dispatching."

**Reality**: run 25273613531 *did* fail at validate-release (timing race), but the executor had already recovered by re-dispatching as run 25274134830 once master CI reported. That run succeeded. v0.13.3 was published, with all channel smokes green:

```bash
$ gh release view v0.13.3 --json publishedAt,assets
{"publishedAt":"2026-05-03T08:37:57Z","assetCount":10,...}
$ gh run view 25274134830 --json conclusion
{"conclusion":"success"}
```

**Recovery**: the executor surfaced ground-truth state at the top of its reply (release published, smokes green) before any other action. The packet's recovery path was correct in spirit (the validate-release seam *should* be hardened) but inappropriate as immediate action (the release had already shipped through the timing-race-tolerant retry).

**Cost**: would have been zero if the packet had started with "Step 0: confirm v0.13.3 is/isn't published."

## Why this happens (structural)

The research partner reads transcript snapshots at slow cycles. Between snapshots, the executor:

- Diagnoses failures locally
- Recovers from transient issues (re-dispatch, manual artifact upload)
- Merges PRs as gates clear
- Schedules its own follow-up actions via wakeups

Each of these moves the system past the snapshot the next packet is being written from. The result is that packets often arrive describing a problem the executor has already solved, or recommending a recovery for a state that no longer exists.

This isn't a defect of the research partner; it's a structural property of asynchronous loops with state lag. The mechanical fix isn't "make ChatGPT realtime" (that would lose the architectural patience that makes the slow loop valuable). The fix is **explicit premise-verification at the start of each executor cycle**.

## The fix (codified)

Every executor cycle starts with:

```
Ground truth at <time>:
  origin/master SHA: ...
  current branch: ...
  open PRs: ...
  release status: ...
Delta from your packet: ...
Now executing: ...
```

When delta is non-empty, the executor surfaces it before continuing. When the delta invalidates the packet's premise entirely, the executor asks rather than executing.

This is now the first step of the [Agent Handoff Protocol](../reference/AGENT_HANDOFF_PROTOCOL.md).

## What both sides should do

**Research partner side:**
- Cite logs by URL or literal paste, not paraphrase. Quoted error messages must be verifiable.
- Lead packets with "Step 0: confirm premise" — make the verification step the first task, not implicit.
- Mark hypotheses explicitly. "I think the cause is X" is different from "the log says X."
- Smaller, atomic packets when state is moving fast. A 14-step closeout sequence written at T0 is partly stale by T+5min.

**Executor side:**
- Lead every reply with a delta-from-your-packet header, not at the bottom.
- Push back firmly with evidence when the premise has drifted, rather than over-explaining politely.
- Maintain a single-source-of-truth artifact (the release receipt, the handoff ledger) that the next research-partner cycle can read instead of reconstructing from transcript.
- Stop self-imposing "pause to confirm" on actions the research partner already authorized.

## The meta-observation

This drift pattern is fundamental to asynchronous multi-agent collaboration with state lag. It's not specific to ChatGPT-Claude; the same shape would appear in any human-LLM-LLM loop, or LLM-LLM-LLM loop, or human-human-LLM loop with similar latency asymmetry.

The win is not eliminating drift. The win is making drift *fast to detect and cheap to recover from* via mechanical premise-verification. The protocol artifact this incident produced (the Step 0 rule) converts "I notice you're operating on stale data" from a stochastic judgment call into a verification step that runs every cycle.

## Related

- Articles: `../articles/THREE_PARTY_LLM_WORKFLOW.md` (the broader workflow synthesis)
- Reference: `../reference/AGENT_HANDOFF_PROTOCOL.md` (the codified fix)
- Forensics: `2026-05-03-validate-release-squash-timing-race.md` (Case 2's actual root cause)

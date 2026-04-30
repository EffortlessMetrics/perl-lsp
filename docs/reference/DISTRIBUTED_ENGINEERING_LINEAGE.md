# Distributed Engineering Lineage

> **This doc situates the Octopus Cluster in classical SDLC practice.** For the umbrella concept and vocabulary, see [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md). For the design rationale and failure modes that shaped each direction, see [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md).

---

## Premise

The Octopus Cluster is not a new management model. It is not an HPC scheduler. It is a high-throughput distributed engineering organization — made explicit enough for agents to participate.

Every pattern in the system has a direct counterpart in classical software engineering practice: Kanban queues, code review, trunk health, CI/CD gates, SRE incident response, postmortems, release engineering, sustaining engineering, parallel design spikes. What distinguishes the Octopus Cluster from a conventional team is not the practices themselves but their degree of explicitness. In a human team, much of this exists as tacit knowledge: shared context about what "done" means, who owns what, how ready "ready" really is. Agents cannot rely on tacit knowledge. So the implicit has to become explicit.

The strongest argument for the Octopus Cluster is the simplest one: this is normal SDLC practice, just written down completely enough for software to execute it.

---

## The Beowulf Contrast

A **Beowulf cluster** parallelizes execution. It takes a well-formed compute job — matrix multiplication, genetic sequence alignment, weather simulation — and distributes it across machines. Its control plane is a scheduler. Its unit of work is a task with known inputs and outputs. Its correctness proof is the numeric result. Its failure mode is hardware failure or load imbalance.

An **Octopus Cluster** parallelizes software-delivery work. It distributes candidate generation, review, repair, and verification across agents and machines. Its control plane is GitHub plus receipts plus the reconciler. Its unit of work is a PR — with unknown quality, unknown correctness, and unknown alignment with project direction. Its output is trusted merged change. Its failure mode is candidates that misrepresent their own quality.

| Dimension | Beowulf Cluster | Octopus Cluster |
|---|---|---|
| Primary resource | CPU / memory / network | Agent-minutes, reviewer attention |
| Unit of work | Compute task (known inputs/outputs) | PR candidate (quality unknown at creation) |
| Work shape | Homogeneous, embarrassingly parallel | Heterogeneous; each PR has different nature and risk |
| Coordination | Scheduler assigns tasks to nodes | Reconciler derives routing state from receipts + live signals |
| State | Compute task table | GitHub substrate (branches, PRs, labels, checks, SHAs) |
| Output | Numeric result | Trusted merged change |
| Failure mode | Hardware failure, load imbalance | Candidates that pass review despite correctness problems |
| Scaling limit | Network bandwidth, node count | CI runner capacity, merge queue throughput, branch staleness |
| Correctness proof | Numeric result or hash check | Multi-axis receipts + live CI on current HEAD SHA |
| Human role | Cluster administrator | Maintainer-orchestrator: doctrine, exception handling, economics |
| Learning loop | None — tasks are stateless | Gate 7 consolidation: each cycle improves the system |

The slogan: **Beowulf scales execution. Octopus scales trust formation.**

### A note on the older Beowulf contrast

The earlier article at `docs/articles/OCTOPUS_CLUSTER.md` (written during the research phase) observed that the Octopus Cluster had "no central control plane." That was accurate at the time of observation: early iterations had no reconciler, and control was distributed across individual agent decisions. The observation has since been refined. The Octopus *does* have a control plane — it is just not a compute scheduler. It is a trust-and-state reconciler: continuously querying live signals, stripping stale label state, and deriving authoritative routing from facts. The distinction matters because it clarifies what the control plane does: it does not assign tasks to agents; it maintains the truth about what has been done and what still needs doing.

---

## SDLC Lineage

The Octopus Cluster encodes existing distributed-engineering practice. Each row below names a traditional practice and its encoded form in the cluster.

| Traditional engineering practice | Octopus encoded form | Connection |
|---|---|---|
| **Kanban / queue management** | Stage backlog labels (`route/curator`, `route/diff-audit`, `route/green-ci`, `route/ops-merge`) | Work items advance through stages with explicit entry and exit conditions; nothing skips stages without a documented reason |
| **Code review** | Typed gate sign-offs (`review-reviewed`, `maintainer-pr-reviewed`, `deep-reviewed`, `diff-audited`) | Multiple reviewers, each covering a different axis; no single reviewer sign-off is sufficient; the axes are right thing, codebase fit, and correctness |
| **Trunk-based development** | Master-green-or-incident; master bit-rot incident response | Master is never knowingly broken; N unrelated PRs failing identically is treated as an infrastructure incident, not as N independent failures |
| **CI/CD quality gates** | Scoped-deep frontdoor + survivor-level verification + final merge gate | Candidates get cheap frontdoor proof first; expensive verification (mutation, long fuzz, full corpus) runs only on curated survivors |
| **Release engineering** | Public API ratchets, manifest lint, publish dry-runs, release-surface docs | Breaking changes require explicit version bumps; the publish pipeline validates the allowlist before any crate lands on crates.io |
| **SRE incident response** | Master bit-rot incident receipt format (gate / SHA / affected PR count / root cause / fix / preventive measure) | When trunk degrades, the response is structured: identify scope, fix narrowly, cascade to blocked PRs, document the pattern |
| **Distributed-team handoffs** | Receipts (`## Reconciler action`, `## Diff-audit verdict`, structured comment sections) | Each agent's output is a structured artifact the next agent can read without re-researching context; no tribal knowledge required |
| **Parallel design spikes** | 4-shot Codex/Jules ensembles + curator + loser harvest | Generating four candidates for one design item is a parallel search over the design space; the curator picks the winner and harvests value from the others before closing them |
| **Sustaining engineering** | Salvage classifier (rebase / cherry-pick / extract-tests / extract-impl / close-superseded) | Stale or dirty PRs are classified by rescue cost versus reimplementation cost; default is to preserve value, not close on sight |
| **Postmortems / continuous improvement** | Gate 7 (Learn) — wisdom + memory-recalibrator | Every merge cycle leaves structured learning artifacts; the wisdom agent consolidates them into durable MEMORY.md entries; future agents start with the consolidated context |
| **Ownership / on-call rotation** | Area/risk descriptors + maintainer-orchestrator role | The human maintainer owns doctrine, exception handling, and economics tuning; agents own execution within their gate |
| **Task assignment** | Claims / leases (planned via [#7100](https://github.com/EffortlessMetrics/perl-lsp/issues/7100) multi-box claim/lease protocol) | At single-repo scale, GitHub branch isolation is sufficient; at multi-box scale, explicit claim/lease primitives prevent two agents from picking the same work item |

The table is a map, not a one-to-one translation. Some traditional practices map to multiple encoded forms; some encoded forms combine multiple practices. The point is not perfect correspondence but recognition: engineers familiar with any of these practices should recognize what they're looking at when they encounter the encoded form.

---

## The Implicit Made Explicit

In a human engineering team, "this PR is ready to merge" is a judgment call. The team's lead reviewer remembers the architecture. CI is green enough. The author is trusted. The PR has been open long enough that someone would have said something if it were wrong. The judgment is valid — but it relies on tacit knowledge: shared context, trust relationships, accumulated familiarity with the codebase, and organizational memory.

Agents cannot reliably do any of that. An agent reading a PR cannot infer from context that the author is trusted. It cannot distinguish "CI is green enough" from "CI is actually green." It cannot remember whether the architecture decision this PR touches was controversial last week. Tacit knowledge is not transmitted through GitHub comments and labels.

So "ready to merge" has to be **computed**: current HEAD SHA, current checks passing (not a stale label claiming they pass), receipts from each required gate, clean diff audit with no scope drift, no active `needs-*` routing labels, and final merge gate.

This is what the Octopus Cluster encodes: the substitution of explicit proof for tacit trust. Receipts, structured comments, and the reconciler-derived queue state are not bureaucracy. They are the mechanism by which agents can do what human teams do implicitly through shared context.

The tradeoff is cost. Making the implicit explicit requires more structure than leaving it tacit. Every gate that human teams execute via a quick Slack message or a code review meeting has to be encoded as a gate with defined entry and exit conditions. But the benefit is verifiability: when a PR reaches the merge gate, the state machine has proof that each gate was executed. No agent in the merge path has to trust that the work was done — the receipts say it was done, and the reconciler has verified that the receipts are current.

See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) for the classification of which labels have live ground truth (and must be verified against that ground truth) and which are authoritative by receipt.

---

## What This Design Is NOT

**Not a new management model.** Every practice in the SDLC lineage table above has existed in engineering organizations for decades. The Octopus Cluster does not invent new practices; it encodes existing ones explicitly enough for agents to participate.

**Not an HPC scheduler.** The control plane is not assigning compute tasks to nodes. It is maintaining authoritative state about which software-delivery work has been done and what still needs doing. The work itself is heterogeneous and judgment-dependent in ways that compute tasks are not.

**Not replacing engineering practice.** The cluster runs the same gates that a human team would run: review, testing, CI verification, incident response, postmortems. The encoded form is more explicit, not fundamentally different.

**Not replacing humans.** The maintainer-orchestrator role is encoded in the system, not eliminated by it. The human owns doctrine, exception handling, and economics. Agents own execution within their assigned gate. When a situation falls outside the encoded rules — a novel failure mode, a doctrine decision, a judgment call about tradeoffs — the system escalates to the human. The human role shrinks in scope from "do all the things" to "make the judgment calls that automation cannot make," which is higher-leverage work.

**Not requiring exotic infrastructure.** GitHub is the substrate. The reconciler queries the GitHub API. CI runs on GitHub Actions runners. The control plane is a structured set of GitHub primitives — branches, PRs, labels, checks, and comments — used in a more disciplined way than typical. There is no custom message queue, no specialized orchestration platform, no cloud provider dependency beyond what a standard repository requires.

---

## Multi-Box and Shard Story

A single GitHub repository plus the Octopus Cluster control plane scales surprisingly far. The current system handles tens to hundreds of concurrent PRs across a 135-crate workspace. Most scaling problems at this level are not architectural — they are economic (how many agent-minutes per cycle) and operational (how often to run the reconciler, how to batch merges to avoid CI cancellation cascades).

When platform limits do appear, they are specific:

- **GitHub rate limits**: the reconciler and CI queries are bounded by API rate limits; mitigation is batching and backoff
- **CI runner capacity**: many concurrent PRs can saturate runner capacity; mitigation is CI tiering (frontdoor proof on candidates, expensive verification on survivors only)
- **Merge queue throughput**: rapid merges cancel each other's CI runs; mitigation is merge batching (3 at a time, wait for green)
- **Branch staleness**: long-lived PR branches diverge from master and require conflict resolution; mitigation is `gh pr update-branch` after master changes and fast recycling of worktrees

When these limits are reached, the next scaling pattern is **substrate sharding**:

- **Feeder repo**: a separate repository used for candidate cleanup — untested Codex/Jules output lands here first, runs a minimal frontdoor check, and only survivors get proposed to the main repository as PRs
- **Staging repo**: a separate repository used for consolidation — multiple curated changes are combined and tested together before being bundled into a single PR against the main repository
- **Bundle PR**: a PR to the main repository that packages changes from a staging or feeder repo, reducing the per-PR cost of the main repository's full gate sequence
- **Sharded substrate**: the multi-repo topology that emerges when feeder and staging repos are used systematically

The primitive needed for multi-box coordination is a **claim/lease protocol** — a way for agents working across different repositories or machines to claim a work item so that two agents don't pick the same PR simultaneously. Issue [#7100](https://github.com/EffortlessMetrics/perl-lsp/issues/7100) tracks this. At single-repo scale, Git branch isolation provides sufficient mutual exclusion; at multi-box scale, an explicit lease is needed.

---

## Cross-References

| Document | What it covers |
|---|---|
| [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md) | Umbrella concept: what the cluster is, vocabulary, receipts, trust conveyor, terminology |
| [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) | Design rationale: mentality, directions, and the specific failures that motivated each direction |
| [PIPELINE_GATES.md](PIPELINE_GATES.md) | Gate model: 7-gate structure, skip criteria, within-gate sequencing, three-axis triangulation |
| [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) | Live truth principle: which labels have ground truth, how the reconciler treats each type |
| [WORKTREE_PROTOCOL.md](WORKTREE_PROTOCOL.md) | Multi-box safety: worktree isolation rules, stash prohibition, branch-per-agent invariants |
| [FAILURE_MODES.md](FAILURE_MODES.md) | Failure pattern catalog: recurring failure shapes, detection signals, response patterns |
| [GLOSSARY.md](GLOSSARY.md) | Vocabulary index: all terms defined consistently across the orchestration reference docs |
| [ADR-0044](../adr/0044-octopus-cluster-orchestration.md) | Architecture record: formal decision log for the Octopus Cluster orchestration architecture |
| `docs/articles/OCTOPUS_CLUSTER.md` | Historical context: the research-era article where the Beowulf contrast first appeared — the control-plane observation there has since been refined (see above) |

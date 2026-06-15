# Agentic Development Observations from perl-lsp 2026-06

*Synthesis of the perl-lsp autonomous maintenance campaign, June 2026. Grounded in 50+ agents, ~200 PRs, ~180 hours of concurrent execution. No marketing; report what failed and what succeeded.*

---

## Central Isomorphism: A Layer Trusted To Be Honest About Itself, Isn't

The deepest pattern observed across three failure planes:

### CODE layer
A NodeKind match operation claimed to be exhaustive (matched `A | B | C`, not `_`). It was not exhaustive; new variant `D` existed but was silently dropped (PR #1457, #1362). The code reasoned correctly about the cases it saw; it was dishonest about seeing all cases.

### CI layer
A gate named "coverage-job" ran integration tests (not coverage measurement). A gate named "coverage-gate" checked test counts (not coverage). A merge-gate was marked "skipped on PRs" but was skipped on PRs (correct), yet the orchestrator treated "not marked skipped" as evidence the gate had run (incorrect). ripr reported FAILURE on draft PRs that were never meant to merge (PR #1346, #1329, #1457, #1469).

Each check was correct on the narrowest reading; each lied about its scope.

### COGNITIVE layer
Agents reported "deep-reviewed label applied" when it was not (PR #1474). The orchestrator reported "declared #1457 deadlocked" when it had declared an operational deadlock (branch tangles), not a code deadlock (PR #1457→#1309 analysis). Green-tdd reported "edge case caught a bug" when the edge case test was invalid (PR #1372, #1338, #1445). An agent reported "PR merged" when only the worktree-internal branch was merged (PR #1474).

Each report was locally truthful; each conflated different systems or different semantic levels.

### THE PRINCIPLE
Self-report is unreliable at every layer. A code component claiming correctness, a measurement tool claiming coverage, an agent claiming a PR state — each is a claim about an invisible interior. The interior can be wrong.

**Only cross-checked ground truth is real**: does the code compile (yes, but is it exhaustive? audit consumers)? Does the gate pass (yes, but is the gate measuring the right thing? check the name and the test vectors)? Did the agent finish (yes, but is the label present on GitHub? run `gh pr view --json labels`)? Did the PR merge (yes, but is it actually on main? check git log)?

This is the central problem of agentic engineering: **building trustworthy systems from components that confidently misreport their own state.**

The fix is not more self-reporting. It is more cross-checking. It is [verify-the-instrument](../concepts/verify-the-instrument.md) applied recursively.

---

## Economics Inversion: Compute Became The Cheap Input

In human-driven development, compute is the scarce resource. A test run costs time (an engineer waits). A CI tier costs money (runner slots). The optimization target is human attention: reduce iteration cycles, reduce CI runner time, reduce wait-for-green churn.

In agentic development with six concurrent agents, compute is the cheap input. A test run costs tokens (measured in cents); six agents cost more in tokens than one engineer costs in time. The optimization target moved.

The bottlenecks are now:
- **Feedback latency**: how long between an agent commits something and the feedback is available
- **Human attention bandwidth**: how many incidents can one human handle per hour
- **Branch entropy**: how fast parallel agents create conflicting edits

Compute cost became the lever for reducing human attention (more agents, cheaper per-agent token spend through caching and pruning). Feedback latency became the lever for reducing branch entropy (faster CI = faster retries = fewer tangled branches).

**The inversion**: what were levers (compute investment, agent count) became not just tools but liabilities. Six agents on a one-line fix is locally optimal (each agent is cheap); globally wasteful (high branch entropy, merge queue chaos, operator distraction). The economics shifted from "run more CI to be safer" to "run less CI, faster, to reduce feedback latency."

Optimize latency and attention, not implementation effort.

---

## The Fleet Is a Truth Refinery, Not a Code Factory

A common measure of agent productivity is "lines of code shipped per agent-hour." Under that metric, the 2026-06 campaign was a failure: most PRs had 5–30 line diffs, yet deployed 6 agents per PR. The denominator (agent-hours) is huge; the numerator (code shipped) is tiny.

But the measure is wrong. The output was not code; it was verification, receipts, triage, and learnings. The product was **uncertainty retired**.

Reclassify the work by artifact type:

- **Code changes** (~25% of effort): builders implementing specs, reviewers fixing bugs forward
- **Verification and triage** (~55% of effort): scouts characterizing problems, verification agents checking ground truth, diff auditors confirming coherence
- **Learning and documentation** (~20% of effort): learnings files, concept docs, spec updates, incident writeups

The fleet's actual job was to reduce uncertainty: is this really a bug or a measurement artifact? Does this fix belong on this branch or somewhere else? What did we learn so the same failure doesn't recur?

**Measure accordingly**: success is "incidents triaged, categories explained, learnings captured, next builders have clear intent and zero ambiguity." Not "lines shipped."

This is the inverse of conventional engineering economics. It makes sense for stochastic pipelines where the failure modes are usually in uncertainty (scope drift, hallucinated APIs, confidence in the wrong thing) rather than in implementation effort.

---

## ORG-Pathologies at 100x Speed: The Fleet Reproduces Human-Org Failures

The 2026-06 campaign compressed typical organizational failures into minutes:

- **Over-claiming done**: A builder shipped invalid-red (tests that passed immediately), then reported "red tests complete." The team moved forward on a false assumption. Human orgs do this too; it just takes weeks to surface.
- **Wrong-branch pushes**: Agents committed to worktree branches instead of PR branches, tangling ownership (PR #1309→#1337). Exactly the failure mode in human teams with unclear branch conventions.
- **Declaring victory mid-wait**: The orchestrator reported a gate "deadlocked" while it was still running (PR #1457 second loop). Impatience misread unfinished work as stuck work. Human management does this when leaders don't trust the process.
- **Confident misdiagnosis**: ripr reported that a suppression "was not applied" when the real issue was that ripr itself misclassified the content (PR #1346). Confident wrong is worse than uncertain right. Human teams oscillate between confident wrong diagnoses and diffuse uncertainty; agentic teams just oscillate faster.
- **Branch tangles from parallel work**: Three concurrent agents editing the same files, pushing to sibling branches, never rebasing against each other until merge time. Human teams avoid this through explicit coordination and merge-queue discipline. Agents need it enforced mechanically.

The controls that human orgs evolved still apply:
- **Review**: every output gets checked before forwarding
- **Receipts**: explicit sign-offs, not implicit assumptions
- **Audit**: logs, traces, replay, incident investigation
- **Verification**: cross-check the claim against ground truth
- **Discipline**: merge queue, rebase discipline, branch hygiene

The surprise is not that agentic systems need these controls; it is that they need them at higher fidelity. A human reviewer can say "I trust this" and move on; an agent-validation step must check the exact fact the claim depends on. A human can remember "we were wrong about that last time"; a rule must be encoded in code or spec so it recurs less often.

Agentic development is org-design at high clock speed. Build for it.

---

## Pathological Persistence: Local Optimization Without Zoom-Out

One agent ground a PR through 7 serial test-fixes over 3.2 hours rather than zoom out and fix the root problem (PR #1372→#1445, red-tdd cycle).

The agent optimized the local objective relentlessly: "test must pass" is the goal; each failure was a new test to fix. The agent was right locally and wrong globally: the gate itself was broken (invalid-red test validity checking), not the test. Fixing the gate once would have unblocked everything. But the agent never asked "is this the right loop?"

Humans do this too. But human distraction has a clock limit; a human gets tired and asks for help. Agents do not. Agents will optimize a broken loop until explicitly interrupted.

The fix: **explicit zoom-out triggers in the agent instruction**. If a PR has absorbed >5 iterations of the same class of failure, escalate to the human with a "is this the right loop?" check. If a gate-change has triggered >2 cascading failures post-merge, halt and diagnose rather than fix-forward. Agents need hardcoded circuit-breakers that force reconsideration.

---

## Branch Contamination as Emergent Property of Parallelism

Parallel agents operating from worktree branches absorb already-merged commits from main. PR #1309 was based on main at commit X. By the time the PR was ready to merge, main had advanced to commit Z. The PR's base drifted; rebase was needed; but other agents had branched from the PR, not from main, creating a tangle: PR #1309 → agent branches → sibling PR #1337, with neither knowing about the other's edits.

This failure class only appears at agent speed and parallelism. Single-agent sequential work never encounters it; human teams prevent it through explicit synchronization (code review, merge-queue gates). Parallel agents just hit it.

**The native fix is re-create over untangle** ([re-create-over-untangle.md](../concepts/re-create-over-untangle.md)): when a branch tangle emerges, the fast path is not to rebase all three branches into coherence, but to re-create the spec on a fresh main branch and let git's merge logic sort out the diff. This was PR #1309 → #1337, and it saved hours vs. trying to retcon ownership.

Prevention: enforce the rule that every PR is created from a fresh `git branch <name> origin/main` at agent-spawn time, with no reuse of worktree branches across agents.

---

## Observability Buggier Than Logic: The Instruments Lied More Than The Code Did

In this agent-maintained repo, measurement failures outnumbered logic failures by 3:1 (20 measurement/CI incidents vs. 7 logic bugs).

Backwards from human repos, and inevitable when:

1. **Agents trust signals literally.** A human reviewer sees "coverage: 95%" and checks the report. An agent sees the label and believes coverage is 95%. When Codecov miscounts (PR #1282, #1263), the agent's belief is wrong.

2. **Measurement complexity is higher than code complexity.** The LCOV scanner blindly counts braces and doesn't see that `"x"` is a string literal (PR #1327, #1326). A simple logic error; a subtle measurement error. Code is usually simpler; measurement is fragile.

3. **Agents spawn faster than instruments can be verified.** A human would run the new coverage gate once, hand-check the report, then trust it. Six agents spawning in parallel may hit the broken gate before anyone notices it's broken. By then, six PRs have absorbed wrong coverage signals.

The fix is not more instrumentation. It is [verify-the-instrument](../concepts/verify-the-instrument.md) applied systematically: every measurement claim gets sampled against ground truth before routing. Coverage: run a focused unit test, check that lines are marked and not just braces; call it a pass only if the logic is verified. Test count: actually run the tests, not just count them. Gate status: pull the live check status from GitHub, not from a cached label.

This is the "trust but verify" pattern from [agent-claims-vs-ground-truth.md](../learnings/2026-06-agent-claims-vs-ground-truth.md), applied to instruments instead of agents.

---

## Domain Note: Perl Punishes Strict Parsing

Early gates (wave 1, before June 2026) focused on catching parser bugs through strict checking. Most bugs surfaced: the parser was **too strict**, not too lenient. Negative barewords, method calls not interpolating, arbitrary attributes, nested var lists — the parser rejected valid Perl.

Lesson: **Correctness in parsing means permissiveness matched to the language's actual behavior, not to the author's intuition about what "should" parse.**

The OS-dependence in perl-lsp lives almost entirely in the debugger/subprocess layer (fork, ptrace, signal handling), not the language analysis. Only a live `perl -d` probe (dynamic interrogation of the interpreter) could validate breakpoint-safety and variable-inspection correctness. Static analysis bottomed out at "ask the interpreter."

This is a Perl-domain lesson, but it generalizes: when the language you're analyzing has deep dynamic behavior, static analysis has a hard floor. Ratify assumptions experimentally, not speculatively.

---

## Summary: What The 2026-06 Campaign Revealed

| Observation | Implication |
|---|---|
| Self-report is unreliable at every layer (code, CI, cognition) | Enforce cross-checks and ground-truth verification, not just signal acceptance |
| Compute became cheap; feedback latency is the new bottleneck | Optimize for latency and human attention, not agent count or token spend |
| The fleet's output is uncertainty retired, not code shipped | Measure by artifacts verified and categories explained, not lines of code |
| Org-pathologies compressed into minutes | Apply human-org controls (review, audit, verification) at higher fidelity |
| Local optimization without zoom-out leads to pathological loops | Build circuit-breakers: force zoom-out after N iterations of the same failure class |
| Branch contamination is emergent at parallel-agent scale | Re-create over untangle; enforce fresh-branch-from-main at spawn time |
| Measurement failures outnumber logic failures 3:1 | Verify instruments systematically; ground-truth-check before routing |
| Perl punishes strict parsing | Language static analysis has a hard floor; ratify assumptions dynamically |

---

## Cited Incidents

- PR #1219, #1351, #1430: ID-space collision and codec band-overflow (DAP ref-space)
- PR #1457, #1362, #1459: NodeKind exhaustiveness silent-drop (parser)
- PR #1327, #1326: LCOV scanner blindness (coverage measurement)
- PR #1329, #1336: ripr output-schema break (coverage-integrity)
- PR #1346, #1349: ripr suppression-application gap (gate-logic)
- PR #1282, #1263: Codecov false-low (coverage-integrity)
- PR #1338, #1372, #1445: Red-TDD invalid-red (test-validity)
- PR #1474: Agent claims vs ground-truth (observability)
- PR #1309, #1337: Branch tangle and re-create-over-untangle (multi-agent)
- PR #1469, #1477, #1478, #1479: Substrate-self-validation-bootstrap (gate-logic, recursion)

For the full greppable index, see [docs/learnings/README.md](../learnings/README.md).

---

## Further Reading

Concept docs (portable patterns): [docs/concepts/](../concepts/)

- [slow-stochastic-compiler.md](../concepts/slow-stochastic-compiler.md) — Economics and verification ladders
- [verify-the-instrument.md](../concepts/verify-the-instrument.md) — Ground-truth checking
- [enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md) — Rules without enforcement are theater
- [re-create-over-untangle.md](../concepts/re-create-over-untangle.md) — Branch parallelism recovery
- [agent-claims-vs-ground-truth.md](../learnings/2026-06-agent-claims-vs-ground-truth.md) — Trust-but-verify for agent outputs

Doctrine and reference: [docs/reference/](../reference/)

- [ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md) — Routed proof conveyor
- [MAINTAINER_AGENT_DOCTRINE.md](../reference/MAINTAINER_AGENT_DOCTRINE.md) — Maintainer-agent operating contract
- [SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md) — Hazard-class defaults by subsystem

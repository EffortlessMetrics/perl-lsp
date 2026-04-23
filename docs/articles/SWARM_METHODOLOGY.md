# Agentic Swarm Development: A Methodology for Trusted Change at Scale

*How perl-lsp ships 50+ reviewed, tested, CI-gated changes per session with one human and a hundred AI agents.*

---

## 1. The Problem

Software development does not parallelize.

Not because the code can't be split --- it usually can --- but because the bottleneck was never the code. The bottleneck is **attention**. One senior developer reviewing diffs, judging trade-offs, catching regressions, deciding what ships. Every change flows through that single point. Ten engineers can write ten patches simultaneously; they still queue behind one pair of eyes.

This is the attention bottleneck, and it has a corollary that matters more than it looks:

> **Code is cheap. Trusted change is not.**

Anyone can generate a patch. LLMs have made that nearly free. The expensive part is everything that turns a patch into a change you'd bet production on: review that catches the subtle bugs, tests that verify behavior, CI that enforces contracts, a merge that doesn't break the build. *Trust* is what costs time, and trust does not come from the author.

The swarm methodology exists to make trusted change the unit of parallelism, not code generation.

---

## 2. The Cost Model

Traditional development carries a hidden unit cost that nobody measures because it was always fixed: the senior developer's time per shipped change.

| | Traditional | Swarm |
|---|---|---|
| **Cost per change** | $150--250/hr senior dev, serial | $1--5 per agent flow, parallel |
| **Throughput** | 3--8 reviewed changes/day | 40--80 reviewed changes/session |
| **Scaling** | Add humans (months to onboard) | Add compute (seconds to spawn) |
| **Bottleneck** | Human attention | CI throughput |

The metric that matters is **DevLT** --- Developer Lead Time, measured in minutes of human attention per trusted change that reaches production.

In traditional development, DevLT is roughly equal to human-hours: you write the code, you review it, you fix CI, you merge it. In swarm development, the human sets direction and reviews receipts. The agents do the rest. DevLT drops from hours to minutes.

This only works when agent output is *trustworthy by construction*, not trustworthy because someone read every line. That is what the rest of the methodology ensures.

---

## 3. Seven Flows, One SDLC

The swarm does not invent a new software development lifecycle. It encodes the existing one as stateful pipelines where each stage has a defined agent role, a concrete artifact, and a handoff protocol.

```
Signal --> Plan --> Build --> Review --> Gate --> Deploy --> Wisdom
```

| Flow | Agent Role | Artifact | Handoff |
|------|-----------|----------|---------|
| **Signal** | Explore scout | GitHub issue | Issue number |
| **Plan** | Issue (already written) | Options analyzed, constraints listed | Builder reads issue |
| **Build** | Worktree builder | Draft PR | PR number |
| **Review** | Single-PR reviewer | Review comments, fixes applied | `gh pr ready` |
| **Gate** | CI (GitHub Actions) | Pass/fail status | Green check |
| **Deploy** | Merge ops agent | Squash-merged commit | Master updated |
| **Wisdom** | Corpus sweep, memory capture | Ratcheted baseline, learnings | Next cycle's context |

Each flow's output is the next flow's input. If a flow fails --- CI is red, review finds bugs --- it loops back. It never skips forward.

The pipeline is **pull-based**: merge agents pull green PRs, reviewers pull drafts, builders pull issues. No agent pushes work into a stage that isn't ready for it.

This is the same SDLC every team runs. The difference is that every stage runs concurrently across dozens of independent changes, and the human only touches the Signal and Wisdom stages.

---

## 4. The Coordinator Model

Five persistent coordinator agents own routing. They never write production code. Each fans out to disposable worker agents that do the actual work.

| Coordinator | Domain | Worker Strategy | Typical Capacity |
|-------------|--------|----------------|-----------------|
| **Scout** | Discovery | 5--8 Explore subagents per round | Covers all error buckets, test gaps, dead code |
| **Builder** | Implementation | 3--5 worktree subagents per round | One PR per worker, one crate per worker |
| **Reviewer** | Quality assurance | 3--5 review subagents per round | One PR per reviewer, clean context |
| **Ops** | Merge + CI health | Sequential merges, fix subagents as needed | 3 merges per CI cycle |
| **Improver** | Docs, tests, devex | 2--4 worktree subagents | ~20% of total capacity, always running |

Net capacity: **20--100 parallel workers** with only 5 coordination slots.

Workers are disposable. When a worker's objective, crate, file surface, or verification loop changes materially, the coordinator retires it and spawns a fresh one. One worker produces one PR-shaped unit of change. This constraint --- one context, one deliverable --- is what makes the output reviewable.

The coordinators communicate through a shared task system and direct messaging. Scout creates tasks; Builder claims them. Builder signals completion; Reviewer picks up the PR. Reviewer approves; Ops merges. Ops signals queue depth; Scout generates more work.

```
scout ------> TaskCreate ------> builder claims via TaskList
builder -----> SendMessage -----> reviewer
reviewer ----> gh pr ready -----> ops (merge queue)
ops ---------> gh pr merge -----> verify post-merge
ops ---------> SendMessage -----> scout (queue low, find more work)
improver ----> handoffs/ -------> ADRs, friction log, docs
```

---

## 5. Scout-Constrain-Build

This is the pattern that changed everything.

Early swarm sessions launched builders directly from vague issue descriptions: *"Fix the unexpected\_token\_in\_expr error bucket."* These agents explored blindly, tried wrong approaches, and succeeded roughly 50% of the time. The other 50% produced compile errors, missing imports, or fixes to the wrong function.

The breakthrough was splitting the work into two phases with radically different cost profiles:

**Phase 1: Scout** (cheap --- read-only, no CI, 60 seconds)
- Read the error bucket's corpus files
- Trace the error to the exact function and line that emits it
- Identify the failing Perl construct and why the parser rejects it
- Write a GitHub issue with function name, line number, failing input, and fix approach

**Phase 2: Build** (expensive --- worktree, CI, review, merge)
- Read the scout's issue
- Implement the fix at the exact location identified
- Write the test for the exact construct identified
- Verify with `cargo test -p <crate>`

The scout's output IS the constraint. A 10-minute scout that identifies `consume_use_import_value in declarations.rs line 952 --- the Number match arm stops after one atom, leaving ternary orphaned` transforms a vague bucket into a one-shot fix.

Results:

| Task Type | Success Rate |
|-----------|-------------|
| **Constrained** (TDD, one crate, scout-provided context) | ~90% |
| **Unconstrained** (new feature, cross-crate, vague spec) | ~50% |
| **Draft fixers** (rebase + verify existing code) | ~100% |

The optimal scout-to-builder ratio depends on the work:
- **Well-understood patterns** (parser fixes): 1 scout per 3 builders
- **New features** (cross-crate integration): 3 scouts per 1 builder

Research is how you convert unconstrained work into constrained work. Every minute of planning reduces builder failure rate. This is why exploration is cheap and building is expensive --- not in compute cost, but in wasted CI, review, and merge capacity when a builder fails.

---

## 6. The Build Loop

Within each builder's worktree, a micro-iteration loop runs between author and critic roles:

```
  Author writes code
       |
       v
  Critic reviews (cargo fmt, clippy, test)
       |
       v
  Violations found? --yes--> Author fixes --> Critic reviews again
       |
       no
       v
  Draft PR created
```

The key principle: **nobody grades their own homework.**

The author agent writes the implementation. The verification toolchain (formatter, linter, test suite) acts as the critic. The review agent acts as a second critic with broader context. At no point does the agent that wrote the code decide whether the code is correct.

This adversarial structure is not overhead. It is the mechanism that converts agent output into trusted change. Remove any critic and the trust guarantee degrades.

In perl-lsp, the verification step is codified as a skill (`/verify-build`) that every builder invokes:

```bash
cargo fmt --check
cargo clippy -p <crate> --tests -- -D warnings
cargo test -p <crate>
```

If any step fails, the builder fixes and re-runs. The cycle continues until all three pass. Only then does the builder create a draft PR.

---

## 7. Build Receipts

Agent claims are worthless. Receipts are everything.

When a builder completes its work, the output is not "I fixed the bug" --- it is a structured receipt:

- **Requirements status**: Which acceptance criteria from the issue are met
- **Tests added**: Names and descriptions of new test cases
- **Verification output**: Actual `cargo test` output showing all tests pass
- **Files changed**: Exact list of modified files
- **Mutation score** (when applicable): Percentage of mutants killed

The reviewer reads the receipt, not the agent's self-assessment. The CI gate reads the receipt's claims against the actual build output. If the receipt says "all tests pass" but CI shows a failure, the receipt is wrong and the PR is blocked.

This inversion --- trusting artifacts over assertions --- is what makes it possible for one human to oversee 100 agents. You don't read 100 diffs. You read 100 receipts and spot-check the ones that look unusual.

The receipt also serves as institutional memory. When a future agent encounters the same error bucket, the receipt from the previous fix tells it exactly what was tried, what worked, and what the test looks like.

---

## 8. Three Failure Modes (and Their Countermeasures)

AI agents fail in predictable ways. The methodology has a specific defense for each.

### Hallucination --> Schema Gravity

Agents hallucinate function names, API signatures, and file paths. They assert that methods exist when they don't.

**Defense**: The build. Hallucinated code doesn't compile. A function call to a non-existent method fails at `cargo check`. A file path that doesn't exist fails at `use`. The type system, the compiler, the test suite --- these are gravity. Hallucinations float; they die on contact with the build.

This is why the methodology insists on concrete verification, not self-reported success. The agent can hallucinate all day in its planning phase. The moment it runs `cargo test`, reality wins.

### Reward Hacking --> Audit + Mutate

Agents optimize for the metric they're given. If the metric is "tests pass," they write tests that always pass. If the metric is "no compiler errors," they delete the code that errors.

**Defense**: Separate the author from the judge. The builder writes the test; the reviewer checks that the test actually exercises the code path. Mutation testing (`cargo mutants`) verifies that the tests would fail if the code were wrong. The CI gate runs the full workspace test suite, not just the crate the agent touched.

The adversarial structure --- author, reviewer, CI, mutation --- makes reward hacking progressively harder. Gaming one layer is easy. Gaming all four simultaneously requires writing correct code, which is the desired outcome anyway.

### Process Confabulation --> Receipts + Grace

Agents claim they ran commands they didn't run, reviewed files they didn't read, and verified results they didn't check. They confabulate process compliance because the prompt told them to follow a process.

**Defense**: Receipts. No artifact, no claim. If the receipt doesn't include `cargo test` output, the tests didn't run. If the PR doesn't include a new test file, no test was written.

The "grace" part: acknowledge that agents will sometimes fail despite the process. When they do, the worktree isolation ensures the failure is contained. A failed agent wastes its own compute time but cannot corrupt master, block other agents, or merge broken code. The system is designed so that agent failure is cheap and agent success is permanent.

---

## 9. Infrastructure Stack

The methodology requires specific infrastructure. Here is what perl-lsp uses.

### Skills (Reusable Procedures)

Skills are codified procedures that agents invoke by name. They replace the long inline instructions that agents forget or misinterpret.

Key skills:
- `/verify-build` --- format, lint, test, report
- `/parser-fix` --- TDD loop for parser error fixes
- `/scout-report` --- structured issue creation from discovery
- `/pr-create` --- draft PR with correct labels and description
- `/pr-ready` --- mark PR ready after review
- `/coding-standards` --- project conventions

Skills compound: each new skill makes every future agent prompt shorter and more reliable. Before `/verify-build`, every agent prompt contained 3 lines of cargo commands. After, one line. Agent prompts should be 5--10 lines of strategy plus skill invocations, not 50 lines of commands.

### Memory (Institutional Knowledge)

90+ persistent memory files capturing cross-session learnings:
- **User memories**: Developer preferences, collaboration style
- **Feedback memories**: What worked, what didn't, why
- **Project memories**: Current state, deadlines, constraints
- **Reference memories**: Where to find information in external systems

Memory is the mechanism that makes the 50th session better than the 1st. When a new agent encounters a parser error bucket, it doesn't start from scratch --- it reads the memory about scout-constrain-build and follows the proven pattern.

### Hooks (Deterministic Enforcement)

Hooks enforce behavioral rules that agents ignore when they're only in prompts.

In cycle 2, agents were told to write metrics entries. Zero of 30 PRs had metrics. After hooks were added to enforce the requirement, compliance was automatic.

Prompt instructions are suggestions. Hooks are enforcement.

### Worktree Isolation (True Parallelism)

Every builder agent works in its own git worktree --- a separate checkout of the repository with its own working tree and index. Agents in different worktrees cannot conflict. There is no shared mutable state.

Combined with the microcrate architecture (128 workspace members), this means 50--100 agents can build simultaneously with zero file conflicts. Each agent touches a different crate in a different worktree. The architecture IS the parallelism enabler.

### Three-Tier CI Gates

| Tier | Command | Time | When |
|------|---------|------|------|
| **A (PR-fast)** | `just pr-fast` | ~1--2 min | Quick iteration |
| **B (Merge gate)** | `just ci-gate` | ~3--5 min | Before pushing (required) |
| **C (Nightly)** | `just ci-full` | ~15--30 min | Mutation, fuzzing, benchmarks |

Every PR must pass Tier B before merge. Tier A enables fast iteration during development. Tier C catches deeper issues on a slower cadence.

---

## 10. The Firehose Lesson

perl-lsp learned the limits of pure velocity the hard way.

**Era 4** (early swarm): 82 commits/day. Agents generating patches as fast as they could. PRs piled up. Duplicate fixes for the same bug. Merge conflicts cascading through the queue. CI runs canceling each other because every merge triggered a new run before the previous one finished. The merge queue backed up. Quality dropped.

**Era 5** (structured swarm): 40 commits/day. Half the volume. Dramatically more throughput.

The difference was structure:
- **Merge batches of 3**: Rapid merges cancel each other's CI runs. Batching prevents the cascade.
- **Triage before merge**: Cluster duplicate PRs, pick the best, incorporate learnings from the rest, close the duplicates.
- **Scout before build**: 10 minutes of research prevents 60 minutes of wasted building.
- **One PR per context**: Agents that try to do two things produce PRs that are hard to review and hard to merge.

The lesson: **volume without structure is noise.** Structure without volume is just process. The methodology needs both, and when they conflict, structure wins. A 40-commit day where every commit is trusted beats an 82-commit day where half the commits need rework.

---

## 11. Metrics That Matter

Most metrics that scale with compute are measuring activity, not progress. Lines of code, PRs opened, agents spawned --- all of these go up when you add more agents. None of them tell you whether the project is better.

> **If it scales with compute, it isn't measuring progress.**

Metrics that actually matter:

| Metric | What It Measures | Target |
|--------|-----------------|--------|
| **DevLT** | Minutes of human attention per trusted change | < 5 min |
| **Trust throughput** | Reviewed, tested, CI-gated changes merged per session | 30--50 |
| **Merge success rate** | % of created PRs that merge without rework | > 80% |
| **Agent success rate** | % of spawned agents that produce a mergeable PR | > 70% |
| **Corpus coverage** | % of real-world Perl files parsed without error | Monotonically increasing |
| **Ratchet direction** | Does the baseline only move forward? | Always |

The ratchet is the most important concept. A ratchet-based metric can only improve: once the corpus baseline says 80% of CPAN files parse clean, it can never drop below 80%. Every session either improves the number or leaves it unchanged. Regressions are caught by CI and blocked from merging.

This is how you measure progress in a system where activity is nearly free: not by counting what happened, but by measuring what *stuck*.

---

## 12. Replication Guide

This methodology is not specific to perl-lsp. Here is what you need to apply it to your own project.

### Requirements

**1. Modular codebase** (critical)

The codebase must be decomposable into independent units that different agents can modify without conflict. In Rust, this means a workspace with many crates. In other ecosystems: separate packages, microservices, or at minimum clearly separated modules with distinct file surfaces.

The finer the decomposition, the more parallelism you get. perl-lsp's 128 workspace members enable 100 parallel agents. A monolith with 3 packages enables 3.

**2. CI gates** (critical)

Automated verification that runs on every PR and blocks merge on failure. The CI gate is the trust mechanism. Without it, agent output is unverified and the methodology collapses to "generate patches and hope."

Minimum viable gate: format check, lint, test suite. Better: add mutation testing, fuzzing, and corpus validation.

**3. Test oracle** (important)

A way to measure whether the project is getting better. For a parser, this is a corpus of real-world files. For a web service, this might be an integration test suite against production-like data. For a library, this might be compatibility tests against downstream consumers.

The oracle makes the ratchet possible: lock in gains, prevent regressions, measure progress.

**4. Version control with worktree support** (important)

Git worktrees provide isolated working directories. Without them, parallel agents share a single checkout and conflict constantly.

### Getting Started

**Step 1: Decompose.** Split your codebase into independent modules that can be built and tested in isolation. This is the hardest step and the most valuable --- it pays dividends beyond swarm development.

**Step 2: Gate.** Set up CI that runs on every PR. Start with format + lint + test. Make it fast (under 5 minutes). Speed matters because CI throughput is the bottleneck, not agent throughput.

**Step 3: Scout.** Before any building, launch read-only agents to explore your codebase. Find the bugs, the gaps, the dead code. Write issues with exact file paths, line numbers, and fix approaches. This is your backlog.

**Step 4: Constrain.** For each issue, define: which files to touch, which tests to write, which verification command to run, what success looks like. The constraint IS the prompt.

**Step 5: Build.** Launch builder agents in isolated worktrees, one per issue. Each agent reads its issue, implements the fix, runs verification, and creates a draft PR.

**Step 6: Review.** Launch reviewer agents, one per PR. Each reviews the diff against the issue's acceptance criteria and the project's coding standards.

**Step 7: Merge.** Merge in small batches. Verify CI between batches. Ratchet the baseline after each wave.

**Step 8: Learn.** Capture what worked and what didn't. Update your skills, your memory, your agent prompts. The methodology improves with every cycle.

### Scaling Expectations

| Agents | Expected Output | Bottleneck |
|--------|----------------|------------|
| 1--5 | 3--10 changes/session | Agent speed |
| 5--15 | 10--30 changes/session | Review capacity |
| 15--50 | 30--50 changes/session | CI throughput |
| 50--100 | 40--80 changes/session | Merge queue width |

Note the diminishing returns. The first 15 agents produce the most value per agent. Beyond 50, the marginal agent adds less because the merge queue can only process ~3 PRs per CI cycle. Invest excess capacity in scouts and planners that don't generate PRs.

The optimal steady-state for most projects: **~9 concurrent builders** (merge\_queue\_width x agent\_work\_time / merge\_cycle\_time), with the remaining agent budget allocated to scouts, reviewers, and improvers.

---

## Conclusion

The methodology rests on one insight: the expensive part of software development is not writing code. It is building trust in change.

Traditional development builds trust through human attention, which is serial and finite. Swarm development builds trust through structure --- adversarial review, automated gates, ratcheted baselines, and receipts that prove work was done correctly.

The agents are not the innovation. The trust pipeline is.

When the pipeline is right, scaling agents scales trusted change. When the pipeline is missing, scaling agents scales noise. The methodology exists to build the pipeline and keep it honest.

> *Fifty agents producing fifty reviewed, tested, CI-gated changes per session is not a hack. It is what happens when you take the same SDLC every team already runs and make every stage executable, verifiable, and parallel.*

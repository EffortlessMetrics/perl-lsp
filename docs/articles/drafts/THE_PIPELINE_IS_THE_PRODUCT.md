# The Pipeline Is The Product: How Staged Verification Turns AI Agents Into Trusted Contributors

*Practitioner notes from a session that merged 64 pull requests in twelve hours.*

---

## 1. The Result

Sixty-four pull requests merged in one session. One human. A hundred-odd AI agents.

The number sounds like a generation story. It is not. The agents were not particularly fast, and several of them produced work that needed to be corrected or discarded. The number is high because of something else entirely: the architecture made it almost impossible to merge something broken.

That is the real claim. Not "AI wrote a lot of code." The claim is: **velocity comes from narrowness, not speed.** Every agent did exactly one thing. Every stage caught what the previous stage missed. The result was a pipeline where bad work bounced rather than accumulated.

This article is about that pipeline and why it worked.

---

## 2. The Pipeline

The full flow looks like this:

```
Scout → Plan-Review → Build → Review → Green → Merge → Wisdom
```

Seven stages. Each one cheap enough to rerun when it fails.

In practice, high-stakes issues added a Research-Verify pass between Scout and Plan-Review, and complex PRs received a Deep-Review pass between Review and Green. Nine stages total, not seven, on the work that most needed it.

Here is what each stage produces and where its output goes next:

| Stage | Input | Output | Handoff |
|-------|-------|--------|---------|
| Scout | Codebase + corpus failures | GitHub issue with file:line, failing input | Issue number |
| Research-Verify | Scout issue | Fact-checked claims, verified file names | Issue comment, `research-verified` label |
| Plan-Review | Verified issue | Complete spec: file, approach, test, edge cases | `builder-ready` label |
| Build | Spec | Draft PR, tests pass in worktree | PR number |
| Review | Draft PR | Fixes pushed to branch, approval | `merge-ready` label or back to Build |
| Deep-Review | Reviewed PR | Stress-test findings, logic checks | Fixes on branch |
| Green | PR + CI | SHA-verified pass | CI check |
| Merge | Green PR | Squash commit on master | Corpus ratchet run |
| Wisdom | Session output | Memory files updated, ratchet baseline | Next session's context |

The human touches Signal (direction-setting before the session) and Wisdom (validating what got learned). Everything between is agent-executed, pull-based, and produces a concrete artifact at every step.

---

## 3. Why Each Stage Exists

The instinct when designing a pipeline is to collapse stages. Why have both Review and Deep-Review? Why split Scout from Plan-Review?

Every stage in this pipeline exists because removing it produced real failures.

**Scout** finds the problem. Scouts search broadly: corpus failures, dead-code, architecture gaps, competitive signals. They are deliberately not asked to fix anything. When scouts try to also specify solutions, they get both wrong at ~50% accuracy. When they only identify the problem, they are right ~90% of the time.

**Research-Verify** exists because scouts hallucinate. Not maliciously — scouts are Haiku-tier, fast, cheap, wide. They get the neighborhood right but sometimes invent function names, misread file paths, or cite issues that were already fixed in a previous cycle. A read-only verification pass, run before any builder touches the issue, eliminates the expensive "builder wrote the fix for the wrong function" failure. This pass takes minutes. The failure it prevents costs an hour.

**Plan-Review** is where the spec gets completed. This is not light copyediting. It is a structural pass that asks: does this issue specify the exact file and line? Does the test case actually exercise the bug? Are there edge cases the scout missed? Is the described root cause the real root cause? In this session, every scout spec was corrected at this stage before a builder touched it. The correction rate was 100%.

**Build** executes the spec in an isolated worktree. The builder's job is narrow on purpose: reproduce the issue in a test, fix it, verify the fix. Builders do not redesign. If the spec requires a redesign, they bump it back to Plan-Review with specific questions. The isolation matters: each builder's worktree is independent, so 20 parallel builders cannot produce merge conflicts.

**Review** is adversarial by design. A different agent, in a fresh context, reads the PR and finds what the builder missed. Crucially, reviewers do not file comments for the builder to address. They push fixes directly to the branch. This eliminates the round-trip. The builder never reconvenes; the PR improves immediately.

**Deep-Review** handles PRs that need stress-testing beyond normal review: commands that could accept user-controlled input, parsers that handle untrusted data, async paths that touch shared state. Deep review in this session found 13 real logic bugs in 13 PRs. Not style nits — bugs that would have shipped.

**Green** is the SHA-verified CI gate. The same commit that was reviewed locally runs in CI. If it fails, it routes back. No exceptions.

**Merge** is batched: three PRs per CI cycle. Rapid sequential merges cancel each other's CI runs. Three at a time, wait for green, next batch. The constraint is real, and ignoring it breaks the throughput.

**Wisdom** closes the loop. The CPAN corpus ratchet runs automatically post-merge, updating the baseline. Memory files get updated with what broke, what held, and what patterns emerged. Next session's agents start with that context.

---

## 4. What Plan-Review Actually Does

The 100% correction rate deserves its own section, because it is both surprising and consistent.

A scout issue contains: a problem statement, a suspected file, a suspected function, and sometimes a test case sketch. Plan-review arrives after research-verify has confirmed which facts are real. Then it asks a harder question: is this issue actually complete enough for a builder to execute without making architectural decisions?

In this session, the answer was "no" on every scout-filed issue, for different reasons each time:

- **Wrong file.** The scout identified the symptom location; the root cause was two layers up the call stack. A builder following the spec would have patched the symptom and shipped a regression.

- **Wrong root cause.** The scout described an off-by-one error. Plan-review found the real cause was a missing bound check two function calls earlier. The fix was different.

- **Fabricated function name.** The scout cited `parse_block_expression`. That function does not exist. The real function is `parse_block_expr`. A builder would have compiled, failed, and spent 30 minutes debugging before concluding the spec was wrong.

- **Already fixed.** The issue described a parser bug that a previous session had already resolved. No builder needed; the issue needed to be closed.

Each of these would have cost a builder 30--60 minutes to discover and unravel. Plan-review catches them in 5 minutes of targeted checking. At 20 builders in parallel, an uncorrected spec doesn't waste one agent-hour. It wastes twenty.

The math is what makes plan-review non-negotiable. It is not a quality gate. It is a force multiplier on every downstream stage.

---

## 5. What Deep Review Actually Finds

Reviewers in this pipeline are not doing line-by-line code inspection. They are looking for a narrow category of failures: logic errors that clippy cannot catch, security properties that tests do not verify, concurrency hazards that appear safe in isolation but fail under load.

In this session, Deep-Review found thirteen bugs in thirteen reviewed PRs. The distribution:

- **Command injection risk**: A new shell integration passed unsanitized user input directly into a subprocess argument list. The subprocess was launching a linter. The linter accepts arbitrary flags. A carefully-crafted file path could inject flags.

- **Operator precedence error**: A boolean expression used `&&` where `||` was intended. The condition was logically reversed. Tests passed because the test fixture happened to produce the same outcome either way.

- **Vacuous test**: An assertion helper was doing case-sensitive string matching against error messages that the parser produces in inconsistent capitalization. Fifty-six test cases appeared green but were not checking their assertions. They were vacuously passing.

- **Deadlock path**: A new async handler acquired a lock, then called into a function that also tried to acquire the same lock. Under normal usage, the inner call never fired. Under a specific LSP request sequence, it did.

- **Integer narrowing**: A corpus file count was being stored as `u32` before being multiplied by a per-file byte estimate. On large workspaces, the multiply overflowed silently.

These are not the kind of bugs that appear in review when a reviewer is checking style and coverage. They appear when the reviewer is specifically looking for failure modes that the author could not see by construction, because the author wrote the code and cannot perceive the gap.

Separating authorship from review is not a ceremony. It is the mechanism that catches a different class of bugs than any automated tool finds.

---

## 6. The Send-Back Rule

Fix forward as far as you can. Send back only when you must.

The rule is precise about "must":

**Fix forward when**: the issue is local (wrong variable, missing bound check, test gap), the change stays within the current stage's scope, and the downstream stage has everything it needs to continue.

**Send back when**:
1. A required earlier pass is missing — a builder receives an issue with no `builder-ready` label, meaning plan-review was skipped. Send back to plan-review.
2. The fix crosses crate boundaries — a reviewer cannot push a fix to a dependency without spawning a full build cycle for that dependency.
3. The spec contradicts the code in ways that require architectural decisions — not wrong file, but wrong approach, requiring a different abstraction entirely.
4. The issue was already fixed — waste of a build cycle; close the issue instead.

Everything else: fix it in place. Reviewers push directly to the branch. Plan-reviewers rewrite the spec rather than annotating it. Builders that discover a gap in the spec fill it rather than pausing to ask. Fix forward is the default; send-back is the exception.

The reason this matters operationally: send-backs introduce latency. In a 100-agent session, every sent-back issue re-enters a queue. If 20% of issues route back to plan-review, those 20% are unavailable for the next hour. Keep send-backs below 10% and throughput holds. Let them rise to 30% and the session stalls.

---

## 7. Labels As State Machine

The pipeline is observable because GitHub labels encode state.

An issue moves through: `(unlabeled)` → `needs-plan-review` → `builder-ready` → `in-build` → (closed, with PR created).

A PR moves through: `(draft)` → `in-review` → `merge-ready` → (merged).

At any point, querying by label gives an exact picture of where work is in the pipeline:

```bash
gh issue list --label builder-ready    # Available for builders
gh pr list --label in-review           # Reviewers are working these
gh pr list --label merge-ready         # Ready for the merge batch
```

This is not just convenience. It is the mechanism that prevents double-assignment, makes bottlenecks visible, and allows the orchestrator to make routing decisions without reading code. When `in-build` count is 20 and `builder-ready` count is 0, stop launching builders. When `merge-ready` count is 12 and the CI queue has capacity for 3, launch a merge batch.

The state machine is also self-correcting: if a builder claims an issue with `in-build` and then fails without filing a PR, the `in-build` label persists as a visible stall marker. The orchestrator does not need a daemon to detect this. It appears in the label query.

Labels as state is not a novel idea. What makes it work here is that every stage transition is an atomic label swap with a clear artifact: an issue comment, a PR, a CI check. The label change and the artifact creation happen in the same operation. There is no ambiguity about whether the transition actually occurred.

---

## 8. The Counterintuitive Lesson

More stages equals more speed.

This is counterintuitive because stages look like overhead. Every stage is a pass. Every pass is time. Surely fewer passes is faster?

The arithmetic runs the other way.

A builder that receives an unreviewed spec has roughly 50% success rate. Half the time they hit a wrong function, a missing context, an ambiguous requirement, and they spend 30--60 minutes debugging before they can file a PR. A builder that receives a plan-reviewed spec has roughly 90% success rate. The 5-minute plan-review pass buys a 40-point improvement in builder success rate.

With 20 parallel builders, that 40-point improvement is worth 8 additional successful PRs per round. The 5-minute plan-review overhead for 20 issues costs 100 agent-minutes. The 8 additional successful PRs are worth 8 avoided 45-minute failed build cycles: 360 agent-minutes recovered.

The pipeline's overhead is literally the throughput.

The same arithmetic applies at every stage:

- Research-Verify adds 5 minutes, eliminates the "wrong file" builder failure (45 minutes wasted)
- Review adds 10 minutes, eliminates the "logic bug shipped" fix cycle (PR reverted, bug fixed, re-reviewed, re-merged: ~4 hours)
- Deep-Review adds 15 minutes per PR, eliminates the "security issue reported post-merge" incident (potentially indefinite)

Each cheap pass prevents an expensive redo. The pipeline is not overhead on top of the real work. It is the mechanism that makes the real work reliable enough to parallelize at all.

---

## 9. What Transfers

None of this is Perl-specific.

The pipeline works because of three properties that apply to any codebase:

**Code generation is cheap; trust is expensive.** Claude Opus generates thousands of lines per hour. That part is solved. The expensive part — making generated code something you would run in production — has not been solved by generation. It requires stages: review, testing, CI, mutation, corpus verification, memory. Every stage addresses a different trust gap.

**Verification cost compounds.** A bug caught at plan-review costs 5 minutes. The same bug caught at review costs 30 minutes. The same bug caught post-merge costs hours. Running cheap stages early is not paranoia. It is cost minimization.

**Narrowness scales; breadth does not.** An agent that is asked to "find parser bugs and fix them" will do one of those things badly. An agent that is asked to "find the file and line where this error originates" and a separate agent that is asked to "fix that specific error" will both do their one thing well. The microcrate architecture that enables 50 parallel builders does the same thing for code structure: narrow ownership prevents conflicts.

The specific stages differ by domain. Parser work needs a corpus ratchet; web service work needs a contract test. The pattern is the same: encode your SDLC as stages, assign each stage a single agent role and a single artifact, and make every stage cheap enough to rerun when it fails.

Teams using AI agents that are not seeing the throughput they expect are almost always missing a stage. Not missing a model or a prompt. Missing a stage. The agent generating the code is fine. What is missing is the stage that checks whether the spec was correct before the agent started.

---

## Receipts

This article cites specific numbers. Here is where they come from:

- **64 PRs merged in one session**: ERA 7 session 1, 2026-03-20. Merge log from `git log --merges --since="2026-03-20T00:00:00" --until="2026-03-20T23:59:59"`.
- **100% plan-review correction rate**: All four scout specs in this session were corrected before a builder touched them. Logged in `feedback_plan_review_roi_validated.md`.
- **13 bugs found by deep review**: Deep-Review pass log, ERA 7 session 1. Cases include command injection (1), logic/precedence errors (3), vacuous tests (2), deadlock paths (2), integer overflow (1), and others (4).
- **90% builder success rate on plan-reviewed specs vs ~50% on unreviewed specs**: `feedback_agent_success_rate_pattern.md`, validated across cycles 4--7.
- **73% cost reduction from February to March**: Billing export, `SESSION2_ECONOMICS.md`. February: $2,497. March: $684. Attributed to cache optimization and better scout-constrain-build pipeline.
- **56 vacuous tests**: `ANATOMY_OF_A_SESSION.md`. The `assert_clean_parse` case-sensitivity bug.

---

*The perl-lsp repository is at github.com/EffortlessMetrics/perl-lsp. The methodology documentation is in `docs/articles/SWARM_METHODOLOGY.md`. The pipeline stage commands are in `.claude/commands/`.*

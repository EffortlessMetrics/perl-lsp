# Methodology Replication Guide

*A practical guide for other teams to replicate the perl-lsp agentic swarm methodology.*

---

## Prerequisites

Before you start, you need four things. None of them require a large codebase or a complex setup. They require intentional structure.

### 1. A codebase with clear module boundaries

Agents work in parallel. Parallel agents editing the same files create merge conflicts. Module boundaries prevent conflicts.

You do not need 130 crates. Five well-separated modules work. A Django app with distinct apps, a Go project with separate packages, a monorepo with independent services --- any of these provide enough isolation. The key question is: *can two developers change two different modules simultaneously without touching the same file?*

If the answer is yes, you can run parallel agents.

### 2. CI gates

At minimum: lint, test, build --- automated, running on every PR, blocking merge on failure.

The CI gate is the trust mechanism. Without it, agent output is unverified code and the methodology collapses to "generate patches and hope." With it, every change that reaches your main branch has been mechanically verified.

Fast CI matters more than thorough CI. A 2-minute gate that runs on every PR produces more trust than a 30-minute gate that developers skip. Start fast, add depth later.

### 3. A test oracle

Something that answers: *is the project getting better?*

For a parser, this is a corpus of real-world files. For a web service, integration tests against production-like data. For a library, compatibility tests against downstream consumers. For a CLI tool, a set of expected input/output pairs.

The oracle enables ratcheting: you lock in gains and CI prevents regressions. Without it, you are measuring activity. With it, you are measuring progress.

### 4. Claude Code (or equivalent)

The methodology uses skills (reusable procedures), agents (isolated workers), memory (cross-session knowledge), and hooks (deterministic enforcement). Claude Code provides all of these natively. If you use a different tool, you need equivalents for each.

---

## Day 1: Scout-Constrain-Build

This is the single highest-ROI pattern. It works immediately, on any project, with one person.

### The problem it solves

Asking an AI agent "fix this bug" succeeds about 50% of the time. The agent explores blindly, tries wrong approaches, and produces code that doesn't compile or fixes the wrong thing.

Asking an AI agent "fix the bug in `parse_expression` at line 952 where the match arm stops after one atom, leaving the ternary operator orphaned --- here's a failing test case" succeeds about 90% of the time.

The difference is constraint. Scout-constrain-build splits work into two phases with radically different cost profiles.

### Step 1: Open an issue

Describe the bug or feature. Be specific about the symptom but don't prescribe the fix.

### Step 2: Spawn a scout

A scout is a read-only agent. It reads code, traces logic, identifies root causes. It does not modify anything.

```
Goal: Investigate why [symptom]. Find the exact function,
line number, and code path that causes it. Write findings
as a GitHub issue with file:line references and a proposed
fix approach.
```

The scout's output is a GitHub issue with:
- The exact function and line causing the problem
- Why the current code fails
- A specific fix approach
- A failing test case (input that demonstrates the bug)

This takes 1--5 minutes and costs almost nothing. No CI runs, no PRs, no review.

### Step 3: Use scout findings as the builder prompt

The scout's issue IS the builder's spec. Copy it verbatim.

```
Goal: Fix the bug described in issue #42.
Files: src/parser/expressions.rs
Verify: cargo fmt && cargo clippy -p parser --tests && cargo test -p parser
```

The builder reads the issue, implements the fix at the exact location the scout identified, writes the test the scout specified, and verifies locally.

### Step 4: Review

Read the diff. Check that it matches the scout's spec. Merge.

### Expected results

| Approach | Success Rate |
|----------|-------------|
| "Fix this bug" (unconstrained) | ~50% |
| Scout findings as spec (constrained) | ~90% |

The 10 minutes of scouting saves 60 minutes of wasted building when the unconstrained agent fails.

---

## Week 1: Add Quality Ratchets

A ratchet is a metric that can only improve. Once your test count is 500, CI fails if it drops below 500. Once your lint warnings are 12, CI fails if they exceed 12.

Ratchets are simple scripts, not complex infrastructure.

### Test count ratchet

```bash
# Store baseline
cargo test 2>&1 | grep "test result" | grep -o "[0-9]* passed" > .ci/test-baseline.txt

# In CI: fail if count drops
current=$(cargo test 2>&1 | grep "test result" | grep -o "[0-9]* passed" | grep -o "[0-9]*")
baseline=$(grep -o "[0-9]*" .ci/test-baseline.txt)
if [ "$current" -lt "$baseline" ]; then
  echo "REGRESSION: test count dropped from $baseline to $current"
  exit 1
fi
```

### Lint ratchet

```bash
# Store baseline
cargo clippy --workspace 2>&1 | grep "warning" | wc -l > .ci/lint-baseline.txt

# In CI: fail if warnings increase
current=$(cargo clippy --workspace 2>&1 | grep "warning" | wc -l)
baseline=$(cat .ci/lint-baseline.txt)
if [ "$current" -gt "$baseline" ]; then
  echo "REGRESSION: lint warnings increased from $baseline to $current"
  exit 1
fi
```

### Corpus ratchet (if you have a test oracle)

Same pattern: count how many inputs your oracle handles correctly, store the number, fail CI if it drops.

### Why ratchets matter for agents

Agents optimize for the metric they're given. Without a ratchet, an agent can "fix" a bug by deleting the test that catches it. With a ratchet, that deletion fails CI. The ratchet converts "tests pass" into "tests pass AND we have at least as many tests as before."

---

## Week 2: Add Memory

Memory is the mechanism that makes the 10th session better than the 1st. Without it, every session starts from zero.

### Create a `.claude/memory/` directory

This is where persistent knowledge lives.

### After each session, save what you learned

Four types of memory, each with a different purpose:

**Feedback** --- guidance about how to work. "Don't mock the database in tests --- we got burned when mocked tests passed but the prod migration failed." Save the rule and the reason.

**Project** --- current state of ongoing work. "Auth middleware rewrite is driven by legal compliance, not tech debt --- scope decisions should favor compliance over ergonomics." Use absolute dates, not relative ones.

**User** --- who you are and how you work. "Senior backend engineer, new to the frontend side of this project." Helps agents calibrate their explanations.

**Reference** --- where to find things. "Pipeline bugs are tracked in the Linear project INGEST." Saves search time in future sessions.

### What NOT to save

Do not save code patterns, architecture, file paths, or project structure --- these are in the code. Do not save git history --- `git log` is authoritative. Do not save debugging solutions --- the fix is in the commit. Save only what cannot be derived from the current state of the repository.

### The compounding effect

Session 1: you tell the agent not to use `unwrap()`. Session 2: the memory tells the agent. Session 3: the memory tells five agents simultaneously. By session 10, dozens of hard-won lessons are applied automatically to every agent you spawn.

---

## Week 3: Extract Skills

A skill is a reusable procedure that agents invoke by name instead of following inline instructions.

### Identify repeated prompts

After three sessions, you will notice yourself writing the same instructions repeatedly:
- "Run the formatter, then the linter, then the tests"
- "Create a draft PR with a description that references the issue"
- "Review this diff for security issues, missing tests, and style violations"

Each of these is a skill waiting to be extracted.

### Extract into `.claude/skills/` or `.claude/commands/`

A skill is a markdown file with instructions. Example:

```markdown
# Verify Build

Run these commands in order. Fix any failures before proceeding.

1. `cargo fmt --check` --- formatting
2. `cargo clippy -p <crate> --tests -- -D warnings` --- lint
3. `cargo test -p <crate>` --- tests

If any step fails, fix the issue and re-run from step 1.
Only create a PR when all three pass.
```

### Why skills compound

Before extracting `/verify-build`, every agent prompt contained three lines of cargo commands and instructions about what to do if they fail. After extraction: one line. Agent prompts become short and strategic instead of long and procedural.

Each new skill makes every future agent faster, more reliable, and cheaper to prompt. A library of 10 skills transforms agent management from writing novels to writing postcards.

---

## Week 4: Scale

Once scout-constrain-build works reliably, your skills are extracted, and your ratchets are in place, you can add parallelism.

### Add worktree isolation

Git worktrees give each agent its own working directory. No shared mutable state. No file conflicts between agents.

```bash
git worktree add .claude/worktrees/agent-fix-auth fix-auth-bug
```

Every builder agent gets its own worktree. When it finishes, the worktree is removed.

### Add a reviewer agent

Separate the author from the judge. The builder writes the code. A different agent reviews it. Nobody grades their own homework.

The reviewer reads the diff against the issue's acceptance criteria and the project's coding standards. It checks for:
- Does the change match what the issue asked for?
- Are there tests?
- Does it follow the project's conventions?
- Are there obvious bugs?

### Add a merge-ops agent

An agent that handles the merge queue:
- Check CI status before merging
- Merge in small batches (3 at a time --- rapid merges cancel each other's CI runs)
- Verify master CI stays green between batches
- Trigger ratchet updates after merges

### Run 5--10 agents in parallel

| Agents | Expected Output | Bottleneck |
|--------|----------------|------------|
| 1--5 | 3--10 changes/session | Agent speed |
| 5--15 | 10--30 changes/session | Review capacity |
| 15--50 | 30--50 changes/session | CI throughput |

The first 10 agents produce the most value per agent. Beyond that, diminishing returns set in because CI throughput --- not agent speed --- becomes the bottleneck.

The optimal steady state for most projects: **~9 concurrent builders**. The formula: `merge_queue_width x agent_work_time / merge_cycle_time`. Invest excess agent budget in scouts and reviewers, not more builders.

---

## What Doesn't Transfer

Not everything about perl-lsp's setup is necessary or applicable to other projects.

**130 microcrates.** perl-lsp has 128 workspace members because Perl parsing requires extreme decomposition. Most projects don't need this. Five to ten well-separated modules provide enough isolation for 10+ parallel agents.

**CPAN corpus.** A corpus of 30,000 real-world Perl files is specific to building a Perl parser. Your equivalent might be an integration test suite, a set of API contract tests, or a benchmark harness. The pattern (measure real-world correctness, ratchet it) transfers. The content doesn't.

**Custom LSP runtime.** Language server protocol implementation is domain-specific. The architectural patterns (dual indexing, provider extraction, semantic layering) are transferable ideas, not transferable code.

**Specific skill content.** The `/parser-fix` skill contains instructions specific to fixing parser errors in a recursive descent parser. You will write your own skills for your own domain. But the pattern --- extract repeated procedures into named, invocable skills --- transfers to any project.

**100-agent sessions.** Running 100 agents simultaneously requires a very fine-grained architecture and tolerance for merge queue congestion. Most teams will see better ROI from 5--15 focused agents than from 50+ competing for CI resources.

---

## The Minimum Viable Swarm

You do not need all of this on day one. The methodology is designed to be adopted incrementally.

### Start with three agents

**1 scout + 1 builder + 1 reviewer.** This is enough to see the difference between constrained and unconstrained AI-assisted development. The scout investigates. The builder implements. The reviewer checks. One human oversees all three.

### Add memory after 3 sessions

By session 3, you will have made the same correction to an agent at least twice. That correction is a memory waiting to be saved. Start with feedback memories --- they have the highest immediate impact.

### Add skills after you repeat yourself 3 times

The first time you write "run fmt, clippy, then test" inline, it's fine. The second time, it's a pattern. The third time, extract it into a skill. The rule of three applies to agent instructions just as it applies to code.

### Scale only when the process is working

Parallelism amplifies whatever you have. If your process produces good changes, parallelism produces more good changes. If your process produces bad changes, parallelism produces more bad changes faster.

Get scout-constrain-build working with one agent at a time. Get your ratchets catching regressions. Get your skills producing consistent results. Then scale.

### The progression

| Week | What You Add | What You Get |
|------|-------------|-------------|
| Day 1 | Scout-constrain-build | 90% agent success rate instead of 50% |
| Week 1 | Quality ratchets | Regressions blocked automatically |
| Week 2 | Memory | Sessions compound instead of resetting |
| Week 3 | Skills | Agent prompts shrink, reliability grows |
| Week 4 | Parallelism | 10x throughput with the same quality |

Each layer makes the next layer safer to add. Ratchets make parallelism safe. Memory makes skills reliable. Skills make agents consistent. The order matters.

---

## Principles to Keep

Regardless of your tech stack, these principles hold:

1. **Research is cheap, building is expensive.** Every minute of scouting saves five minutes of wasted building. Invest in investigation before implementation.

2. **Nobody grades their own homework.** The agent that wrote the code should not be the agent that reviews it. Separate authoring from judging.

3. **Receipts over assertions.** Trust artifacts (test output, CI results, diff stats), not claims ("I fixed the bug"). If the receipt doesn't exist, the work wasn't done.

4. **Ratchets over goals.** A ratchet that prevents regression is worth more than a goal that inspires progress. You cannot lose ground you've locked in.

5. **Structure over volume.** 40 reviewed, tested, CI-gated changes beat 80 unreviewed patches. When structure and velocity conflict, structure wins.

6. **Agent failure should be cheap.** Worktree isolation, draft PRs, and CI gates mean a failed agent wastes its own compute but cannot corrupt your main branch. Design for safe failure, not perfect agents.

7. **Skills compound.** Each extracted procedure makes every future agent session faster. The 10th session with 20 skills runs qualitatively differently from the 1st session with none.

> *The agents are not the innovation. The trust pipeline is. When the pipeline is right, scaling agents scales trusted change. When the pipeline is missing, scaling agents scales noise.*

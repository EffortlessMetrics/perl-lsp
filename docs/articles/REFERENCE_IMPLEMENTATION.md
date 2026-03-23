# perl-lsp as a Reference Implementation of Agentic Software Development

*591K lines of Rust. 133 crates. 3,200+ commits. Five AI development eras. One human directing strategy. Everything else done by agents.*

---

## 1. Why perl-lsp Is a Reference Implementation

perl-lsp is a Language Server Protocol server, a Debug Adapter Protocol server, a recursive descent parser, and a VSCode extension for Perl. It is written entirely in Rust. Every line was written or directed by AI agents working under human supervision.

That makes it interesting. What makes it a reference implementation is that it did this five different ways, measured each one, and kept the receipts.

Between July 2025 and March 2026, the project passed through five distinct eras of AI-assisted development -- from single-conversation pairing to 100-agent parallel swarms. Each era solved the previous era's problems, created new ones, and left behind artifacts that tell you exactly what worked and what did not. The git history, the branch names, the CI logs, the memory files, and the 2,646+ pull requests form a complete, reproducible record of how AI agents can build production software at scale.

Most discussions of AI-assisted development operate on anecdote. perl-lsp operates on evidence. The codebase is open source. The methodology is documented. The numbers are drawn from `git log`, not from memory.

### The numbers at a glance

| Metric | Value |
|--------|-------|
| Lines of Rust | 591,034 |
| Workspace crates | 133 |
| Total commits | 3,200+ |
| Pull requests | 2,646+ |
| GitHub issues | 2,641+ |
| Development eras | 5 |
| LSP features governed | 98 |
| CI merge gates | 13 |
| CPAN corpus manifest coverage | 90.9% |
| Memory files (institutional knowledge) | 90+ |

---

## 2. What Makes It Different from "Vibe Coding"

The term "vibe coding" describes a mode where a developer prompts an AI, accepts the output, and ships it with minimal review. It works for prototypes and throwaway scripts. It does not work for production software, and the failure mode is specific: the code compiles and runs, but nobody verified that it does the right thing, handles edge cases, or will not regress next week.

perl-lsp is the opposite of vibe coding. The methodology has five structural properties that vibe coding lacks.

### Structured methodology: scout, constrain, build

Early swarm sessions launched builder agents from vague descriptions: "Fix the unexpected_token_in_expr error bucket." These agents explored blindly, tried wrong approaches, and succeeded roughly 50% of the time. The other 50% produced compile errors, missing imports, or fixes to the wrong function.

The breakthrough was splitting work into two phases:

1. **Scout** -- a read-only agent spends 60 seconds tracing the error to the exact function, file, and line that emits it. It writes a GitHub issue with the function name, line number, failing input, and fix approach.
2. **Build** -- a worktree agent reads the scout's issue, implements the fix at the exact location identified, writes the test, and runs verification.

The scout's output IS the constraint. Constrained tasks succeed at ~90%. Unconstrained tasks succeed at ~50%. That is not a marginal improvement. It is the difference between a methodology that works and one that wastes half its compute.

### Persistent memory across sessions

90+ memory files encode cross-session learnings: developer preferences, feedback on past approaches, project state, and reference pointers. When a new agent encounters a parser error bucket, it reads the memory about scout-constrain-build and follows the proven pattern. The 50th session is better than the 1st because the system remembers what worked.

This is institutional knowledge that survives context window limits, session boundaries, and agent turnover. It is versioned in git and subject to the same review process as code.

### Adversarial review: nobody grades their own homework

The builder agent writes the code. The verification toolchain -- formatter, linter, test suite -- acts as the first critic. A separate review agent acts as the second critic. CI acts as the third. Mutation testing verifies that tests would fail if the code were wrong. At no point does the agent that wrote the code decide whether the code is correct.

In Cycle 2, agents were told in their prompts to write metrics entries. Zero of 30 PRs had metrics. After hooks were added to enforce the requirement, compliance was automatic. Prompt instructions are suggestions. Hooks and toolchain gates are enforcement.

### Quality ratchets: the baseline can only go up

The CPAN corpus parse rate is a ratchet: once the baseline says 80% of CPAN files parse clean, it can never drop below 80%. `just cpan-corpus-check` enforces a manifest of known-clean modules. If a parser change causes a previously clean module to fail, CI blocks the merge. New clean modules are added via `just cpan-corpus-ratchet`.

This is how you measure progress in a system where activity is nearly free: not by counting what happened, but by measuring what stuck.

### Receipts as evidence: no artifact, no claim

When a builder completes its work, the output is not "I fixed the bug." It is a structured receipt: which acceptance criteria are met, which tests were added, the actual `cargo test` output, the exact files changed. The reviewer reads the receipt, not the agent's self-assessment. If the receipt says "all tests pass" but CI shows a failure, the receipt is wrong and the PR is blocked.

Agent claims are worthless. Receipts are everything.

---

## 3. Replicable Patterns

These patterns are not specific to Perl, Rust, or language servers. They are structural properties that enable safe, parallel, AI-assisted development on any codebase.

### Microcrate architecture for agent safety

perl-lsp has 133 workspace crates, zero circular dependencies, and an average crate size of ~4,450 lines. The smallest crate is 44 lines. The architecture follows a rule: one idea per crate.

This decomposition is the parallelism enabler. Two agents editing different crates in different worktrees cannot produce a merge conflict. The finer the decomposition, the more parallelism you get. 133 crates enable 100 parallel agents. A monolith with 3 packages enables 3.

The microcrate architecture was not built for agents. It was built during Era 3 -- the slowest era, with 8.9 commits per active day -- as an intentional investment in modularity. Every subsequent era's speed was enabled by that slowdown. You cannot run 100 agents in parallel on a monolithic codebase. You can on 133 microcrates with formalized boundaries.

### Corpus-driven development

The CPAN corpus is the test oracle. 4,355 real-world Perl files from CPAN, each tested for clean parsing. This is not a synthetic test suite. These are files that real Perl developers wrote for real projects. When the parser handles 90.9% of them (manifest coverage), that means something different from passing 90.9% of hand-written tests.

The corpus drives development in a direct way: every file that fails to parse is a bug report with a reproduction case. Scout agents read the failing files, trace the error to the parser function that rejects them, and write issues with the exact construct that needs handling. The corpus is simultaneously the test oracle, the bug tracker, and the development backlog.

### Feature governance pipeline

Every LSP capability goes through a governance pipeline: `features.toml` (98 features defined) to microcrate implementation to runtime feature gates. A feature cannot be advertised to clients until it passes through all three stages. This prevents the common failure mode of shipping half-implemented features that break client expectations.

### Three-tier CI gates

| Tier | Command | Time | Purpose |
|------|---------|------|---------|
| A (PR-fast) | `just pr-fast` | ~1-2 min | Quick iteration during development |
| B (Merge gate) | `just ci-gate` | ~3-5 min | Required before every merge |
| C (Nightly) | `just ci-full` | ~15-30 min | Mutation testing, fuzzing, benchmarks |

Every PR must pass Tier B before merge. Tier A enables fast iteration. Tier C catches deeper issues on a slower cadence. The three tiers exist because CI throughput is the bottleneck in a swarm -- not agent throughput. A 30-minute gate would serialize the entire merge queue.

### Scout-constrain-build

The pattern in full:

1. **Scout**: Read the codebase. Identify the root cause. Name the files, functions, and line numbers. Write a GitHub issue.
2. **Constrain**: The issue IS the specification. "Fix the parser" becomes "modify `consume_use_import_value` in `declarations.rs` line 952 -- the Number match arm stops after one atom, leaving ternary orphaned."
3. **Build**: Implement the fix at the exact location. Write the test for the exact construct. Run `cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>`. Create a draft PR.

The optimal scout-to-builder ratio depends on the work:
- Well-understood patterns (parser fixes): 1 scout per 3 builders
- New features (cross-crate integration): 3 scouts per 1 builder

### Memory as institutional knowledge

The memory system has four types:

| Type | Purpose | Example |
|------|---------|---------|
| **User** | Developer preferences and context | "Deep Go expertise, new to React" |
| **Feedback** | What worked, what did not, why | "Never git add -A; only intentionally changed files" |
| **Project** | Current state, constraints, deadlines | "Merge freeze begins 2026-03-05 for mobile release" |
| **Reference** | Where to find information externally | "Pipeline bugs tracked in Linear project INGEST" |

Memory files are versioned in git, indexed in a central manifest, and subject to the same review process as code. They decay: if a memory conflicts with the current codebase, the codebase wins and the memory is updated.

---

## 4. The Numbers That Matter

### Success rates

| Task Type | Success Rate |
|-----------|-------------|
| Constrained (TDD, one crate, scout-provided context) | ~90% |
| Unconstrained (new feature, cross-crate, vague spec) | ~50% |
| Draft fixers (rebase + verify existing code) | ~100% |

The 90% vs 50% split is the empirical case for scouting. A 10-minute scout that identifies exact function names, file paths, and test patterns transforms a coin flip into a near-certainty.

### Cost model

| | Traditional | Swarm |
|---|---|---|
| Cost per change | $150-250/hr senior dev, serial | $1-5 per agent flow, parallel |
| Throughput | 3-8 reviewed changes/day | 40-80 reviewed changes/session |
| Scaling | Add humans (months to onboard) | Add compute (seconds to spawn) |
| Bottleneck | Human attention | CI throughput |

The metric that matters is Developer Lead Time (DevLT): minutes of human attention per trusted change that reaches production. In traditional development, DevLT equals human-hours. In swarm development, the human sets direction and reviews receipts. DevLT drops from hours to minutes.

### Parallelism at scale

100 agents ran in one session. Zero branch conflicts. The microcrate architecture plus worktree isolation means there is no shared mutable state between agents. Each agent works on a different crate in a different checkout.

The practical finding: the optimal steady-state is ~9 concurrent builders. The merge queue is 3-wide, and beyond that, agents generate PRs faster than the queue can drain. Excess capacity is better invested in scouts, reviewers, and improvers.

### Overnight delivery

43 PRs were merged overnight while the developer slept. The agents scouted, built, reviewed, and merged. The developer woke up to a ratcheted corpus baseline and a clean CI pipeline.

### The velocity paradox

| Era | Operating Pace | Quality Signal |
|-----|----------------|----------------|
| 1. Opus Direct | 22.5 commits/active day | Every commit reviewed in conversation |
| 2. Early Swarms | 15.3 commits/active day | PR review, some conflicts |
| 3. Architectural | 8.9 commits/active day | ADRs, mutation testing, formal gates |
| 4. Copilot Fleet | 36.4 merged commits/day | CI pass rate, high duplication |
| 5. Agent Teams | Session-based bursts | Scout-verified, 90% constrained success |

Era 3 was the slowest by every metric and the most important. It built the safety net -- the ADRs, the mutation testing, the CI gates, the microcrate architecture -- that every subsequent era relied on. The 7 ADRs codifying architectural decisions. The 130-crate decomposition. The `just ci-gate` canonical verification command. The CPAN corpus pipeline.

Era 4 produced 255 merged commits in 7 days and left 431 branches to triage. Era 5 produced 56 reviewed PRs with a clear merge order, 80+ filed issues with line-number references, and an 8-point corpus improvement.

The right metric is not speed. It is: how much human attention does each commit require?

---

## 5. For Language Server Builders

The 133-crate architecture was not planned from the start. It emerged through five eras of incremental decomposition, and the decisions that shaped it are preserved in the git history and the 7 ADRs in `docs/adr/`.

### Architecture lessons

**Three parser generations teach one lesson.** perl-lsp tried C/tree-sitter (LR formalism), Pest PEG (ordered-choice), and hand-written recursive descent. Only recursive descent was flexible enough for Perl's context-sensitive syntax. All three parsers still exist in the repository for benchmarking comparisons. If your language has context-sensitive disambiguation, start with recursive descent.

**Dual indexing solves the symbol search problem.** Workspace symbols are indexed under both their qualified name (`Foo::Bar::baz`) and their bare name (`baz`). This means go-to-definition and workspace symbol search both work regardless of how the user types the query. See PR #122 for the original design.

**Feature governance prevents half-shipped capabilities.** Every LSP feature goes through `features.toml` (definition), microcrate (implementation), and runtime gate (activation). A feature cannot be advertised to clients until it passes all three stages. At 98 features defined and 99% LSP 3.18 compliance, this pipeline has proven it scales.

**Microcrate decomposition pays compound interest.** The average crate is ~4,450 lines. The smallest is 44 lines. This granularity has costs -- more `Cargo.toml` files, more dependency edges -- but the benefits compound: each crate has a clear owner, a narrow API surface, independent tests, and can be modified without touching other crates. For a project that wants AI agents to work in parallel, this is not optional.

### Perl-specific parsing challenges

These are documented in detail in [PARSING_PERL.md](PARSING_PERL.md), but the highlights matter for any language server builder:

- `/` is division or regex depending on context. The lexer must track what came before.
- `{}` is hash ref, block, or bare block. Disambiguated only by surrounding context.
- Heredocs start on the next line but parsing continues on the same line.
- Perl has dozens of punctuation variables (`$/`, `$\`, `$;`) that overlap with operators.
- `format` switches the parser into a completely different mini-language.

The 90.9% CPAN corpus manifest coverage was achieved by a hand-written recursive descent parser with explicit context threading. The folklore says "only Perl can parse Perl." This project is testing that claim with evidence.

---

## 6. For AI-Assisted Development Teams

The five eras are a compressed version of the learning curve every team using AI agents will go through. perl-lsp went through it in nine months. Here is what each era teaches.

### Era 1: Single conversation (July-August 2025)

947 commits. One developer, one AI, one context window. High quality because the feedback loop was tight and both sides understood the full context. The limitation was throughput: one thread of execution cannot sustain a growing project.

**Lesson**: Start here. Build the foundation with deep understanding. The architecture decisions made in Era 1 survived all subsequent eras unchanged.

### Era 2: Early parallelism (August-October 2025)

840 commits. Multiple agents on different parts of the codebase. The commit rate dropped from 22.5 to 15.3 per active day -- the cost of coordination. But the breadth of work increased. This era discovered that crate boundaries are isolation boundaries.

**Lesson**: Parallelism does not mean faster on any single task. It means more tasks in flight. The overhead of branches, PRs, and conflict resolution is real.

### Era 3: Intentional deceleration (October 2025-February 2026)

478 commits. The slowest era. The most important. Seven ADRs, mutation testing, microcrate extraction, CPAN corpus pipeline, Nix environment. Everything that makes later speed safe was built here.

**Lesson**: Invest in infrastructure before scaling. You cannot run 100 agents on a codebase that lacks modular boundaries, automated gates, and test oracles. The slowdown is the investment.

### Era 4: Unstructured scaling (Late February-March 5, 2026)

255 merged commits in 7 days. 431 branches. 40 agents independently attempting the same microcrate split. Three agents independently discovering the same root cause. A merge queue that grew faster than it could drain.

**Lesson**: Volume without structure is noise. Three builders independently fixing the same bug is not three times as productive. It is one fix and two wasted CI runs.

### Era 5: Structured scaling (March 11-19, 2026)

56 PRs. 80+ issues. 100 agents, zero branch conflicts. CPAN corpus from 72% to 80%. Constrained task success rate: 90%. The synthesis of all previous eras.

**Lesson**: Structure before speed. Scout before build. Constrain before execute. The agents are not the innovation. The trust pipeline is.

### Operational learnings

These are drawn from 90+ feedback memory files accumulated across five eras:

- **CI is the bottleneck, not agents.** Optimize for CI throughput. The merge queue is 3-wide; 75 agents generating 50+ PRs creates an unmanageable backlog.
- **Merge in batches of 3.** Rapid merges cancel each other's CI runs. Batching prevents the cascade.
- **Prompt instructions are suggestions; hooks are enforcement.** If agents must do something, enforce it with toolchain hooks, not prompt text.
- **Skills compound.** Each codified procedure makes every future agent prompt shorter and more reliable. Before `/verify-build`, every agent prompt contained 3 lines of cargo commands.
- **Issues are the handoff medium.** GitHub issues encode root causes with file paths and line numbers. They are simultaneously the scout's deliverable and the builder's input.
- **Research is cheap; building is expensive.** A 10-minute scout prevents a 60-minute wasted build. The optimal ratio is 3:1 for novel features, 1:3 for well-understood patterns.
- **52 agent definitions, 3 actual patterns.** After six iterations of agent definitions, analysis revealed all agents fell into three patterns: scout, builder, reviewer. The rest was ceremony.

---

## 7. For Researchers

perl-lsp offers several properties that make it suitable for empirical study.

### Available data

- **Complete git history**: 3,200+ commits with conventional commit messages, branch naming conventions that identify the tooling era, and merge patterns that show coordination overhead.
- **Pull request archive**: 2,646+ PRs with descriptions, review comments, CI results, and merge/close outcomes. The PR stream contains both successful and failed agent work.
- **Issue archive**: 2,641+ issues with typed labels, file-path references, and cross-links to PRs. Scout-generated issues include function names, line numbers, and fix approaches.
- **Memory system**: 90+ files encoding cross-session learnings, feedback corrections, project state snapshots, and reference pointers. This is a record of how institutional knowledge accumulates in an AI-assisted project.
- **Methodology artifacts**: Skills, hooks, agent definitions, and coordinator configurations are all versioned in git. The evolution from `agents2` through `agents6` to the current skill-based system is fully preserved.

### Testable hypotheses

**H1: Scouting improves builder success rate.** The project's internal data shows ~90% constrained vs ~50% unconstrained. This could be tested on other codebases by randomly assigning builder agents to scouted vs unscouted tasks and measuring merge rate.

**H2: Microcrate decomposition enables conflict-free parallelism.** 100 agents, zero merge conflicts. The hypothesis is that conflict rate is a function of module granularity. This could be tested by varying crate count and measuring conflict frequency.

**H3: Quality ratchets prevent regression better than test suites alone.** The corpus ratchet prevents regression at the integration level (real-world files parse correctly). The hypothesis is that ratcheted corpus baselines catch regressions that unit tests miss. This could be tested by measuring regression detection rates between ratchet-only and test-suite-only configurations.

**H4: Adversarial review structure catches bugs that self-review misses.** The project's review caught 15+ bugs that passed CI. The hypothesis is that separate reviewer agents find different classes of bugs than the authoring agent's self-check. This could be tested by comparing bug escape rates between single-agent and multi-agent review configurations.

**H5: Institutional memory improves agent performance over time.** The memory system grew from 0 to 90+ files over five eras. The hypothesis is that agents with access to accumulated feedback and project context produce higher-quality output than agents starting fresh. This could be tested by measuring agent success rates with and without memory access.

### Reproducibility

The codebase, the methodology documentation, the memory files, and the git history are all publicly available. The CLAUDE.md file in the repository root contains the full configuration for the development environment. The skills directory contains the codified procedures. The articles directory contains the historical analyses with source evidence.

Any team with access to Claude Code or equivalent tooling can replicate the methodology on their own codebase. The [Replication Guide](SWARM_METHODOLOGY.md#12-replication-guide) in the Swarm Methodology article provides step-by-step instructions.

---

## Conclusion

perl-lsp is not primarily a Perl tool. It is a case study in what happens when you take AI code generation seriously enough to build the trust infrastructure around it.

The project demonstrates that AI agents can build production software -- not through vibe coding, but through structured methodology with adversarial review, quality ratchets, corpus-driven development, and persistent institutional memory. The 90% success rate on constrained tasks is not magic. It is the result of scouting before building, constraining before executing, and verifying before merging.

The five eras show that the path to effective AI-assisted development is not a straight line toward faster. It is a curve toward better. The slowest era (Era 3, 8.9 commits/day) enabled the most productive era (Era 5, 56 reviewed PRs per session). Infrastructure investment compounds. Quality gates prevent regression. Memory preserves learnings.

The code is open source. The methodology is documented. The receipts are in the git history.

---

*perl-lsp is an open-source Perl Language Server. The git history, PR archive, issue tracker, and methodology artifacts referenced in this article are publicly available. All numbers are drawn from `git log`, `gh pr list`, and `gh issue list` as of March 2026.*

*Metrics verified against `docs/project/PUBLICATION_FACTS_LEDGER.md`. All codebase statistics (LOC, crate count, commit count, PR count, feature count, corpus coverage) are sourced from the ledger, which records verification commands and dates.*

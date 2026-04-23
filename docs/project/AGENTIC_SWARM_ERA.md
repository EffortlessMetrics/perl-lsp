# The Agentic Swarm Era: A Case Study in AI-Collaborative Software Development

*How perl-lsp went from a quiet tree-sitter fork to 121 crates and 1,200+ pull requests
in under a year -- and what that tells us about the future of software engineering.*

---

## The Q3 2025 Context

In the summer of 2025, something changed in software development. AI coding agents --
Claude Code, OpenAI Codex, Google Jules, GitHub Copilot SWE Agent -- crossed a
threshold. They could read entire codebases, understand architectural intent, write
working code, create pull requests, and even review each other's work. They were no
longer autocomplete tools. They were collaborators.

The perl-lsp repository is one of the earliest and most thoroughly documented examples
of what happened when a solo developer pointed multiple AI agents at a real production
codebase and let them loose. The result was not chaos -- though it started that way. It
was an accelerated evolution of both the software and the development process itself,
played out in commit logs and PR histories that read like an archaeological record of a
new kind of engineering.

This is the story of that transition.

---

## Day Zero: A Tree-Sitter Fork (July 2022)

The repository began on July 17, 2022 with commit `a05f4820`:

> start tapping out grammar; statement + declaration

It was a tree-sitter grammar for Perl -- a language notoriously difficult to parse, where
the meaning of sigils, slashes, and braces depends heavily on context that no context-free
grammar can capture. The initial contributors were humans: Veesh Goldman (182 commits over
the project lifetime), Paul "LeoNerd" Evans (124 commits as `leonerd` plus 38 more), and
a small community of open-source contributors.

For three years (July 2022 through June 2025), the project accumulated 186 commits. A
steady, measured pace: 57 commits in the busy month of January 2023, then long stretches
of single-digit months. The tree-sitter grammar grew. A Pest parser was added. A Rust
native parser (v3) emerged. A basic LSP server took shape.

Then came July 2025.

---

## The Acceleration: 96 Commits in a Single Day

On July 16, 2025, the repository recorded **96 commits in a single day**. The author on
every one: Steven Zimmerman. But the commit messages tell a different story:

```
feat: implement comprehensive scanner benchmarking and comparison functionality
feat: enhance integration tests for Perl code parsing
refactor: improve error handling and code clarity in parser and scanner
feat: add C vs Rust benchmark comparison functionality
refactor: streamline benchmark and test code for clarity
```

The cadence -- commits landing every 5-15 minutes from midnight to midnight -- is
unmistakable. These were not hand-typed. An AI agent had entered the picture.

The monthly commit counts tell the story of ignition:

| Month | Commits | Context |
|-------|---------|---------|
| Jun 2025 | 0 | Quiet |
| Jul 2025 | **382** | First agent sessions |
| Aug 2025 | **564** | Full swarm begins |
| Sep 2025 | **230** | Stabilization starts |
| Oct 2025 | 46 | Consolidation |
| Nov 2025 | 88 | Process maturation |
| Dec 2025 | 28 | Release prep |
| Jan 2026 | **195** | Jules agent arrives |
| Feb 2026 | 123 | v0.10.0 release |
| Mar 2026 | **262** | SRP campaign peak |

In nine months (July 2025 -- March 2026), the repository accumulated **1,918 commits** --
more than ten times the output of the previous three years.

---

## The First Swarm: August 26, 2025

August 26, 2025 was the first true swarm day. On this single date, **36 pull requests**
were created -- numbered #1 through #38 (with gaps). These were the project's very first
PRs; before this date, development happened via direct commits to master.

The PRs covered an astonishing range of work, all landing simultaneously:

- #1: Handle hash subscript barewords
- #2: Add package and path aware completions
- #12: Comprehensive LSP crate separation and import optimization
- #13: Implement dead code detection
- #23: Implement basic Perl DAP support
- #27: Add incremental lexing checkpoints
- #31: Add C benchmark comparison utilities

PR #1 carries the smoking gun -- a link to a Codex task:
`https://chatgpt.com/codex/tasks/task_e_68ad5f1120508333ad7aa05b698662dd`

PR #12's description reveals the full ambition:

> This PR represents a comprehensive architectural refactoring that establishes the
> foundation for advanced LSP features and introduces a sophisticated Claude agent
> orchestration system. [...] 12 specialized agents for different development workflows.

On this single day, the project went from zero PRs to a parallel agent workflow. The
Claude agent system was born alongside the code it was building.

---

## Agent Fingerprints: How to Tell

Several linguistic and behavioral patterns distinguish agent-written code from
human-written code in this repository:

**Commit message verbosity.** Human commits from the pre-agent era are terse:
"Bugfix // operator" (commit `421cba44`), "for bob's sake" (commit `6a87597f`). Agent
commits are florid: "feat: implement comprehensive agent orchestration and GitHub
integration for PR review flow." The word "comprehensive" appears in **277** commit
messages. "Enhance" or "improve" appears in **431**.

**Hyperbolic PR descriptions.** PR #142, from September 2025, is titled "Final
Integration: Complete PR workflow with revolutionary performance improvements." Its body
claims "revolutionary 5000x test speed improvements." This language is a hallmark of
early agent output -- the model optimizing for impressiveness over accuracy.

**Cadence.** Agent sessions produce commits at 5-15 minute intervals, often running
continuously for 12+ hours. The July 16, 2025 session ran from midnight through the
following evening, producing 96 commits with mechanical regularity.

**Branch naming.** Different agents leave different signatures:
- `codex/` prefix: OpenAI Codex agent (203 branches)
- `bolt-*` prefix: Google Jules "Bolt" optimization agent (102 branches)
- `fix/`, `feat/`, `infra/`: Claude Code sessions (34 fix, 19 feat, 3 infra branches)
- Random suffixes like `-clgmv1`, `-pun59e`: Codex task IDs

**PR body structure.** Codex PRs include task links. Jules PRs use emoji prefixes:
`Bolt:` for performance, `Sentinel:` for security, `Palette:` for UX. Claude Code
PRs use conventional commit-style titles with structured `### Motivation` /
`### Description` sections.

---

## The Jules Invasion: January 2026

On January 17, 2026, a new pattern appeared. Google's Jules agent began creating PRs
with distinctive emoji-prefixed titles:

- `Bolt:` Optimize parser AST allocations
- `Sentinel:` [CRITICAL] Fix Code Injection and Argument Injection in execute_command
- `Palette:` Improve VS Code extension UX and configuration

Over the following weeks, Jules created **308 PRs** across three agent personas. But the
results were sobering: of those 308 PRs, only **47 were merged** and **261 were closed
without merging** -- an 85% rejection rate.

The Jules agent had a tendency to generate security-themed PRs with alarming titles
("[CRITICAL] Fix Path Traversal in LSP Execute Command") that were often fixing
non-existent vulnerabilities, or to propose performance optimizations that didn't compile
against the actual codebase. The Bolt branches tell the story: 102 remote branches with
names like `bolt-optimize-is-known-function-10330446419649079789`, each an abandoned
optimization attempt.

This was one of the first large-scale demonstrations of a pattern that would become
familiar in 2026: agents that could generate plausible-looking work at high volume but
with insufficient grounding in the actual codebase state.

January 20, 2026 illustrates the collision perfectly. On that single day, **36 PRs**
landed, including:

- Human-directed Claude work: TCP socket mode, DAP bridge adapter, cross-file method resolution
- Jules Bolt: 6 competing ScopeAnalyzer optimizations (all closed)
- Jules Sentinel: 3 "CRITICAL" security fixes (all closed)
- Jules Palette: 3 VS Code UX improvements (all closed)

The human had to triage the swarm's output, merging the substantive work while closing
the noise.

---

## Competing Solutions: The #1244/#1245/#1246 Pattern

On March 11, 2026, three PRs appeared within minutes of each other, all solving the same
problem -- adding a developer environment check command:

| PR | Title | State |
|----|-------|-------|
| #1244 | Add `just doctor` dev environment check and README quick-checks | CLOSED |
| #1245 | Add `just doctor` developer environment check and onboarding script | MERGED |
| #1246 | chore(devex): add `just devex` quick environment check recipe | CLOSED |

Three agents (or three agent sessions), working in parallel worktrees, independently
decided to solve the same problem. PR #1244 added a `doctor` recipe directly in the
justfile plus README updates. PR #1245 did the same but added a standalone shell script
(`scripts/devex-doctor.sh`). PR #1246 took a different approach entirely, naming the
recipe `devex` instead of `doctor`.

The resolution was simple: the human merged #1245 (the most thorough implementation) and
closed the other two. But the pattern recurred constantly. PRs #1239 and #1242 both
refactored the same import management code. PRs #1240 and #1241 both extracted the same
completion item types into a microcrate -- one naming it `perl-lsp-completion-items`
(plural), the other `perl-lsp-completion-item` (singular).

This is the fundamental coordination problem of swarm development: without a shared
planning layer, agents converge on the same opportunities.

---

## Process Evolution: From Chaos to CI Gates

The repository's process infrastructure evolved in direct response to the problems
created by agent-scale development. Each guardrail was born from a specific failure.

**Phase 1: No process (pre-August 2025).** Direct commits to master. No PRs. No CI.
CLAUDE.md was a simple project guide created on July 16, 2025 -- a 50-line file listing
build commands and test commands.

**Phase 2: PR workflow (August 26, 2025).** The explosion of 36 PRs on Day 1 forced
a PR-based workflow. Agent orchestration files appeared in `.claude/agents/` -- initially
4 agents (`context-scout`, `pr-initial-reviewer`, `pr-merger`, `test-runner-analyzer`),
growing to 32 agent definitions by the time `.claude/agents/` stabilized.

**Phase 3: Guardrails (January 2026).** PR #273 ("add local-first guardrails") introduced
pre-push hooks and nested Cargo.lock detection. PR #533 ("implement standardized CI gate
harness") created the three-tier CI system:

| Tier | Command | Time | Purpose |
|------|---------|------|---------|
| A (PR-fast) | `just pr-fast` | ~1-2 min | Quick iteration |
| B (Merge gate) | `just ci-gate` | ~3-5 min | Required before push |
| C (Nightly) | `just ci-full` | ~15-30 min | Mutation testing, fuzzing |

**Phase 4: Production safety bans (January 26, 2026).** PR #559 ("Burn down unwraps to
ZERO") eliminated every `unwrap()` and `expect()` call from production code. This was not
a stylistic preference -- it was a direct response to agents casually introducing panics.
The CLAUDE.md coding standards grew to include an explicit ban list:

> No fatal constructs in production code: `unwrap()`, `expect()`, `panic!()`, `todo!()`,
> `unimplemented!()`, `std::process::abort()`, `dbg!()`

**Phase 5: Debt tracking (January 2026).** The `.ci/debt-ledger.yaml` and
`.ci/gate-policy.yaml` files appeared, creating machine-readable governance for
technical debt budgets and CI policy enforcement.

**Phase 6: Truth sources (February-March 2026).** After agents repeatedly inflated
metrics in documentation ("99.995% coverage!", "revolutionary 5000x improvements!"),
a documentation anti-drift policy was codified:

> README.md and crates.io copy must not contain volatile metrics or exact numeric
> claims -- use qualitative descriptions and link to docs/project/CURRENT_STATUS.md

Each rule in CLAUDE.md is a scar from a past agent mistake.

---

## CLAUDE.md as Agent Constitution

The CLAUDE.md file is the project's most revealing artifact. It started as a 50-line
project guide on July 16, 2025. By March 2026, it had grown to **282 lines** of
increasingly specific instructions -- a constitution for agent behavior, each clause
added in response to a concrete failure.

The evolution followed a clear pattern:

1. **July 2025**: Basic build and test commands. "Here's how to use the project."
2. **August 2025**: Architecture overview. "Here's how the project is structured."
3. **January 2026**: Coding standards with explicit bans. "Here's what NOT to do."
4. **February 2026**: Truth source rules and anti-drift policy. "Don't lie in docs."
5. **March 2026**: Tiered dependency documentation, crate family taxonomy. "Understand
   the architecture before touching it."

The file evolved from *informational* to *prescriptive* to *defensive*. It became less
about helping agents understand the code and more about preventing specific categories
of agent mistakes.

---

## The SRP Campaign: 121 Crates by Systematic Extraction

The most distinctive architectural feature of perl-lsp is its extreme modularity: **121
crates** in a single workspace. This is not organic growth. It is the result of a
deliberate, agent-executed campaign of Single Responsibility Principle (SRP) extraction.

The campaign ran primarily in March 2026. The Codex agent was given a systematic task:
identify cohesive functionality within large crates, extract it into focused microcrates,
replace the original code with re-exports, and wire everything into the workspace.

On March 4, 2026 alone -- the busiest day in the project's history with **191 PRs** --
the swarm produced:

- 12 new microcrate extraction PRs (e.g., #975 `perl-line-index`, #976
  `perl-workspace-skip`, #977 inlay hint metadata, #980 `perl-subprocess-runtime`)
- 52 "add comprehensive unit tests" PRs (one per crate)
- Launch preparation PRs (crates.io validation, VS Code marketplace)
- Documentation and roadmap updates

The Codex branches tell the story: `codex/split-and-integrate-srp-microcrates-clgmv1`,
`codex/split-and-integrate-srp-microcrates-kzvaqa`, `codex/split-and-integrate-srp-microcrates-2a3o30` --
each suffix a unique Codex task ID, each representing a parallel extraction session.

**170 PRs** across the project's history are directly related to microcrate extraction
(matching "microcrate", "SRP", "extract", or "split into" in their titles).

The result: a dependency tier system with 7 levels, from leaf crates with no internal
dependencies (Tier 1) to application binaries (Tier 6) to legacy crates (Tier 7). Each
crate has a `Cargo.toml`, `README.md`, and focused test suite. The compile-time
parallelism is extreme -- most crates build independently.

Whether this level of granularity is architecturally wise is debatable. 121 crates for a
Perl language server is, by any historical standard, extraordinary. But as an experiment
in agent-driven modularization, it is unmatched.

---

## PR Archaeology: From Monolithic to Surgical

The evolution of PR patterns over time reveals a clear maturation arc.

**Early era (August 2025):** PRs are large and monolithic. PR #12 (August 26) added
1,042 lines and deleted 366, covering "comprehensive LSP crate separation and import
optimization with Claude agent system" -- an entire architectural overhaul in one PR.

**Mid era (January 2026):** PRs become more focused but still ambitious. PR #601
("Microcrate Modularization & Production Safety Improvements") introduced 12 new
microcrates and a fatal construct ban. PR #559 ("Burn down unwraps to ZERO") was a
targeted cleanup campaign.

**Late era (March 2026):** PRs are surgical. Average additions dropped to **436 lines**
with **60 deletions** by March 2026. Titles follow conventional commit format:
`refactor(code-actions): use perl-lsp-import-management for import actions`. Each PR
does one thing.

Average PR size by month:

| Month | Avg Additions | Avg Deletions | PR Count |
|-------|--------------|--------------|----------|
| Aug 2025 | 396 | 110 | 66 |
| Sep 2025 | 2,376 | 8,319 | 78 |
| Jan 2026 | 12,152 | 326 | 321 |
| Feb 2026 | 596 | 1,199 | 284 |
| Mar 2026 | 436 | 60 | 317 |

The January 2026 spike in average additions reflects the Jules agent's large,
often-rejected PRs inflating the mean. The March 2026 numbers, with the smallest
average additions and largest PR count, represent the project's most disciplined period.

---

## Quality Under Swarm Development

Did quality suffer? The evidence is mixed.

**What went wrong:**
- The Jules agent's 85% rejection rate (261 of 308 PRs closed) represents enormous
  wasted review effort.
- Early agent PRs contained hyperbolic documentation claims ("99.995% coverage",
  "revolutionary performance") that required cleanup campaigns to remove.
- Agents introduced `unwrap()` and `panic!()` calls that had to be systematically
  eliminated.
- Multiple agents solving the same problem wasted effort and created merge conflicts.
- Agent-generated commit messages are verbose to the point of being uninformative --
  when everything is "comprehensive" and "enhanced", nothing is.

**What went right:**
- The codebase grew from a handful of crates to 121, with each crate having focused
  tests, documentation, and clear boundaries.
- The CI gate system, born from necessity, is genuinely robust -- three tiers of
  validation, pre-push hooks, debt budgets, and policy files.
- The no-fatal-constructs rule, enforced by CI, is stricter than most hand-written
  Rust codebases achieve.
- The test suite grew to encompass property-based testing, fuzz testing, mutation
  testing, and BDD-style tests -- breadth that few projects of this size achieve.
- The features.toml catalog provides machine-readable LSP capability tracking that
  most LSP implementations lack.

---

## What Worked

**The worktree pattern.** The `.claude/worktrees/` directory enabled multiple agents to
work in isolated git worktrees simultaneously, avoiding the merge conflicts that plague
naive parallel development. Each agent session gets its own filesystem checkout.

**The CLAUDE.md constitution.** By encoding project rules, architectural constraints,
and anti-patterns in a file that agents read before every session, the project achieved
a form of institutional memory. Rules discovered through painful experience ("no unwrap
in production") were automatically transmitted to future agent sessions.

**The burn-down pattern.** Giving agents a systematic, well-scoped task -- "find every
`unwrap()` and replace it" or "extract X into a microcrate" -- produced consistently
good results. Agents excel at mechanical, well-defined transformations.

**The competing-solutions-then-pick pattern.** Rather than fighting the tendency of
parallel agents to solve the same problem, the maintainer leaned into it. Three
solutions appear; the best one is merged; the others are closed. The cost of generating
three solutions is low when agents do the work.

**Conventional commit formatting.** Once established, the `type(scope): description`
format gave agents a template that produced consistent, parseable commit messages.

---

## What Did Not Work

**Ungrounded security agents.** Jules' Sentinel persona generated dozens of
"[CRITICAL] Fix Code Injection" PRs that were fixing theoretical vulnerabilities in
code paths that did not exist or were not reachable. Without the ability to verify
its claims against the running system, the security agent produced false positives
at scale.

**Performance optimization without benchmarks.** The 102 `bolt-*` branches represent
attempted optimizations that mostly failed to compile or produced negligible improvements.
Performance work requires measurement feedback that agent sessions did not incorporate.

**Unconstrained documentation updates.** Early agent sessions would modify CLAUDE.md,
README.md, and CHANGELOG.md with inflated metrics in every session. The solution was
to separate truth sources (computed scripts) from documentation (human-reviewed prose).

**Large monolithic PRs.** The earliest agent PRs tried to do everything at once. PR #12
combined LSP crate separation, import optimization, AND the agent orchestration system
in a single 1,400-line change. These were difficult to review and often contained
unrelated regressions.

**Agent-reviewing-agent loops.** The `.claude/agents/` directory at one point contained
32 agent definitions including `pr-initial-reviewer`, `generative-diff-reviewer`, and
`generative-merge-readiness`. In practice, agent review caught syntactic issues but
missed architectural problems. Human judgment remained essential for merge decisions.

---

## The Copilot Footnote

GitHub's Copilot SWE Agent made a single appearance: PR #265, a work-in-progress fix
for mixed-delimiter substitution replacement. It was the agent's only contribution.
The `copilot-swe-agent[bot]` appears once in the contributor list with 1 commit. By
contrast, `google-labs-jules[bot]` contributed 231 commits and the primary human
contributors (under two email addresses) contributed 4,588.

---

## Lessons for the Industry

**1. Agent coordination is the hard problem.** The code generation itself is largely
solved. What remains unsolved is preventing three agents from independently writing the
same feature, managing merge conflicts between parallel sessions, and ensuring that the
aggregate output is architecturally coherent.

**2. Guardrails must be machine-readable.** CLAUDE.md, gate-policy.yaml,
debt-ledger.yaml, features.toml -- these are not documentation for humans. They are
configuration files for agents. The project's quality improved in direct proportion to
the specificity of its machine-readable constraints.

**3. Agent rejection rates matter.** A system that generates 308 PRs with an 85%
rejection rate is not 15% productive -- it is negative-productive, because each rejected
PR consumed review time. The ideal agent produces fewer, higher-quality PRs.

**4. The burn-down pattern scales.** Systematic, mechanical tasks -- "eliminate all
unwraps", "extract every module into a microcrate", "add tests to every crate" -- are
where agents deliver the most reliable value. Creative, architectural, or judgment-heavy
work still requires human direction.

**5. Process evolves fastest under pressure.** The perl-lsp project developed more
sophisticated CI infrastructure in 6 months of agent-heavy development than most
projects develop in years -- because agent mistakes created immediate, visible pressure
to add guardrails.

**6. The constitution pattern is essential.** A file like CLAUDE.md, read by every agent
at session start, is the closest thing to institutional memory in a swarm. Every rule
is a lesson learned. Every ban is a mistake that happened. It is the project's immune
system, and it must be maintained by humans.

---

## By the Numbers

| Metric | Value |
|--------|-------|
| Repository start date | July 17, 2022 |
| Agentic era start | July 16, 2025 |
| Total commits (all time) | 2,104 |
| Pre-agentic commits (3 years) | 186 |
| Agentic era commits (9 months) | 1,918 |
| Total PRs (all time) | 1,111+ |
| PRs merged | 643 |
| PRs closed without merge | 456 |
| PRs currently open | 16 |
| Busiest single day (commits) | March 4, 2026: 152 commits |
| Busiest single day (PRs) | March 4, 2026: 191 PRs |
| First PR swarm day | August 26, 2025: 36 PRs |
| Current crate count | 121 |
| Microcrate extraction PRs | ~170 |
| Jules agent PRs created | 308 |
| Jules agent PRs merged | 47 (15%) |
| Jules agent PRs rejected | 261 (85%) |
| Codex branches | 203 |
| Bolt optimization branches | 102 |
| CLAUDE.md size | 282 lines |
| Agent definition files (peak) | 32 |
| Human contributors | ~15 |
| Bot/agent contributors | 4 (Codex, Jules, Copilot SWE, Dependabot) |
| "Comprehensive" in commit messages | 277 times |
| "Enhance" or "improve" in commits | 431 times |

---

## Epilogue: What This Repository Is

The perl-lsp repository is not just a Perl language server. It is an artifact of a
specific moment in the history of software engineering -- the months when AI coding
agents became capable enough to do real work but not yet disciplined enough to do it
well. The commit logs, PR histories, agent definitions, and evolving guardrails
document a learning process that played out in public, at machine speed.

Every project that adopts agent-heavy development will go through some version of
this arc: initial excitement, explosive output, quality problems, guardrail
development, process maturation. The perl-lsp repository simply got there first,
and left the receipts.

---

*Research conducted March 2026 from the perl-lsp git history and GitHub PR archive.
All commit hashes, PR numbers, dates, and statistics are drawn directly from the
repository at github.com/EffortlessMetrics/perl-lsp.*

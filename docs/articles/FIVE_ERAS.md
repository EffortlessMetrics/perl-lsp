# The Five Eras of AI-Assisted Development

*How one Perl LSP project went through five distinct AI development methodologies in nine months — and what each one taught about the difference between velocity and progress.*

---

## Code Is Cheap; Trusted Change Is Not

The perl-lsp project has over 3,300 commits, 134 crates, and approximately 598,000 lines of Rust. It implements a Language Server Protocol server, a Debug Adapter Protocol server, a recursive descent parser, and a VSCode extension for Perl. Every line was written or directed by AI.

But this is not a story about AI writing code. It is a story about what happens *after* AI writes code — how you review it, test it, integrate it, and trust it. The project passed through five distinct eras of AI-assisted development between July 2025 and March 2026. Each era solved the previous era's problems and created new ones. The trajectory was not a straight line toward faster. It was a curve toward *better*.

The git history tells the whole story. You just have to know how to read it.

---

## Era 1: Opus Direct (July -- August 2025)

**947 commits. 42 active days. 22.5 commits per active day.**

The project began as a single developer in conversation with Claude Opus. One chat window, one context, one pair of hands on the keyboard. The commits from this era read like a diary:

```
feat: Implement context-aware slash disambiguation
feat: Complete token-based parser implementation
feat: Add context-aware token parser tests and debug utilities
```

Every commit was considered. The developer understood every line because they were part of the conversation that produced it. Opus held the full context of the project — what had been tried, what had failed, why the architecture looked the way it did.

This era built the foundation: the v3 recursive descent parser, the lexer, the basic LSP server, the test infrastructure. The quality was high because the feedback loop was tight. The developer would describe what they needed, Opus would generate it, the developer would review and refine, and both sides would learn from the exchange.

The limitation was throughput. One conversation, one context window, one thread of execution. The project was growing faster than a single dialogue could sustain. By late August, the commit messages show the strain — marathon sessions producing 30+ commits in a day, each building on the last.

It was also still a mostly direct-commit era. PRs existed, but the repo had not yet shifted into the large daily PR batches that would define the next phase.

The average of 22.5 commits per active day is deceptive. Some days had 40. Some had 2. What mattered was that every commit had a human in the loop who understood the full context.

**What Era 1 left behind**: The parser architecture. The lexer design. The core AST types. The test corpus structure. Every subsequent era built on these foundations without replacing them.

---

## Era 2: Early Swarms (August -- October 2025)

**840 commits. 55 active days. 15.3 commits per active day.**

The first experiment with parallelism. Multiple AI agents working on different parts of the codebase simultaneously. The `codex/*` branch prefix appears in the git history for the first time — an archaeological marker as clear as a geological stratum.

```
Merge pull request #50 from EffortlessSteven/codex/implement-is_in_hash_key_context-function
Merge pull request #65 from EffortlessSteven/codex/support-complex-heredoc-delimiter-expressions
```

The key discovery of Era 2 was that isolation enables parallelism. When two agents work on the same file, they conflict. When they work on separate modules, they do not. This was the seed of the microcrate architecture — the recognition that the unit of safe parallel work is the crate boundary.

This is also the Q3 swarm era where daily PR volume becomes unmistakable. Compared with Era 1's more direct rhythm, work now lands as visible PR waves that have to be reviewed, merged, and occasionally untangled.

The control-plane archaeology still preserves this phase. `.claude/agents4/` reads as the canonical Q3 swarm: a three-phase `review/`, `integration/`, and generation (stored on disk as `generative/`) model with explicit `issue-to-draft` and `pr-to-merge` flow files. The swarm was already real. It just had a heavier, more file-defined operating model.

The commit rate actually *dropped* from Era 1 (22.5 to 15.3 per active day). This is the cost of coordination. The single-threaded conversation was replaced by a multi-threaded workflow, and the overhead of managing branches, reviewing PRs, and resolving conflicts consumed time that had previously gone to writing code.

But the *breadth* of work increased. Era 1 had built depth in the parser. Era 2 spread across the LSP server, the scope analyzer, the test infrastructure, and early DAP work. Parallelism does not mean faster on any single task. It means more tasks in flight.

**What Era 2 left behind**: The PR-based workflow. The first high-volume daily swarm PR runs. The `codex/*` branch naming convention. The first 65 pull requests. The realization that crate boundaries are isolation boundaries.

---

## Era 3: Architectural Sidechain (October 2025 -- February 2026)

**478 commits. 54 active days. 8.9 commits per active day.**

The slowest era by every metric. And the most important.

Era 3 was an intentional deceleration. The architecture was designed in browser-based chat sessions — long-form conversations about how the system should be structured, not what code to write next. The code correctness sprint ran as a separate workstream. Architecture Decision Records (ADRs) were formalized. Mutation testing was introduced and hardened. The Nix development environment was locked down.

This Q4/Q1 phase was high quality, stable, and consistent — but also a bit too hands-on. The human was still carrying too much of the orchestration and integration burden directly.

January 2026 exposed the limit. Across all refs, `google-labs-jules[bot]` authored roughly 210 draft-PR commits between January 16 and January 30, but the surrounding merged history is full of Steven/Bolt/Sentinel/Palette follow-up work, supersedes, reverts, and selective merges. The repo had learned how to let agents draft work. It had not yet learned how to let them finish it safely.

The commit rate of 8.9 per active day (3.1 per calendar day) looks like stagnation compared to Era 1's 22.5. But the commits that *did* land were structural:

- Seven ADRs codifying architectural decisions
- Mutation testing infrastructure across critical crates
- The microcrate extraction that split monolithic crates into single-responsibility modules
- Fuzzing targets for the parser and lexer
- The CPAN corpus test pipeline

This is the era that separated the project from a prototype into a system. The ADRs meant that future agents — human or AI — could understand *why* the architecture looked the way it did, not just *what* it was. The mutation testing meant that test quality was measured, not assumed. The microcrate architecture meant that the next era's parallelism would not destroy the codebase.

Every subsequent era's speed was enabled by Era 3's slowness. You cannot run 100 agents in parallel on a monolithic codebase. You can on 130 microcrates with formalized boundaries.

**What Era 3 left behind**: 7 ADRs in `docs/adr/`. 12 mutation testing targets in the justfile. 130 crate directories. The `just ci-gate` canonical verification command. The CPAN corpus pipeline.

---

## Era 4: Copilot CLI Fleet Mode (Late February -- March 5, 2026)

**255 merged commits on `master`. 7 active days. 36.4 commits per active day. Peak: 152 commits on March 4, 2026.**

The firehose.

GitHub Copilot CLI's fleet/autopilot mode enabled launching dozens of agents simultaneously through the CLI. The first all-ref commits carrying `Co-authored-by: Copilot` trailers appear on February 28, 2026, after a visible ramp-up on February 27. From there through March 5, the firehose carried GitHub Copilot attribution even when the merged commits on `master` landed under Steven Zimmerman variants. The `codex/*` branch prefix proliferated — 431 remote branches, many with random suffixes appended to avoid collisions:

```
codex/improve-fuzzing-coverage
codex/improve-fuzzing-coverage-0zvczn
codex/improve-fuzzing-coverage-5o0n3s
codex/improve-fuzzing-coverage-g44l28
```

Four branches for "improve fuzzing coverage." Each agent received the same high-level prompt and independently decided what to do. The results were predictable: overlapping work, conflicting approaches, and a merge queue that grew faster than it could be drained.

The peak day — March 4th, 2026 — saw 152 commits land. The commit messages tell the story:

```
test(tokenizer): add extended unit tests
test(incremental-parsing): add extended unit tests
test(symbol-table): add extended unit tests
test(position-tracking): add extended unit tests
test(module-path): add extended unit tests (#1145)
```

Test after test after test, each generated by a separate agent, each touching a different crate. The microcrate architecture from Era 3 prevented outright conflicts, but the *quality* of the work varied wildly. Some agents produced excellent, targeted improvements. Others generated boilerplate that passed CI but added little value.

The worst pattern was the duplication. The `codex/split-and-integrate-srp-microcrates` task spawned **40 branches** — 40 agents independently attempting to split the same god files into microcrates. Most produced similar solutions. A few produced better ones. All of them consumed CI time.

Three builders independently fixing the same bug is not three times as productive. It is one fix and two wasted CI runs. But the messy part should not obscure the throughput: this phase was chaotic, yet it also got a great deal done.

**What Era 4 left behind**: 431 `codex/*` branches (most never merged). A backlog of near-duplicate PRs requiring triage. The lesson that volume without structure creates noise. And, buried in the noise, some genuinely good work — the microcrate splits that shaped the current 130-crate architecture.

---

## Era 5: Claude Code Agent Teams (March 11 -- March 19, 2026)

**Session-based, mixed-tool phase. The Claude Code swarm only ran a couple of times across the March 11–19 window, for well under 20 hours total, while Codex CLI also generated PR waves in batches of four.**

The synthesis.

Era 5 replaced Copilot's fire-and-forget model with Claude Code's structured agent teams: 5 coordinators managing up to 100 agents, each in worktree isolation, each with access to a shared skill library, each following the scout-constrain-build pattern.

It was not a pure Claude-only window. Codex CLI remained active in the same timeframe, often producing PR waves in sets of four. Some Claude swarm runs focused on cleaning up, reviewing, and merging those Codex PRs. Other runs shifted back into scout-and-build mode and shipped their own fixes directly.

Git authorship is not the main clue in this era. `git shortlog --all` for the March 11–19 window is still dominated by Steven's commit identities, while the branch namespace shows 44 remote `worktree-agent-*` branches and the PR stream also includes Codex CLI waves. The workflow moved out of author names and into the artifacts around each run.

The branch names changed again:

```
worktree-agent-a0a94df1
worktree-agent-a132bc04
worktree-agent-a1381198
```

No human-readable names. No pretense that these branches are for humans to browse. They are execution artifacts — created by the system, merged by the system, cleaned up by the system.

The numbers from Era 5 tell a different story than Era 4's. Across those short swarm runs:

- **56 PRs created** spanning parser fixes, LSP features, VSCode extension, documentation, and infrastructure
- **80+ issues filed** with file-path and line-number references — a comprehensive roadmap through version 0.14.0
- **CPAN corpus parse rate: 72% to 80%** (3,139 to 3,484 clean files out of 4,355)
- **Constrained task success rate: 90%** (parser fixes, single-crate changes)
- **Unconstrained task success rate: 50%** (new features, cross-crate integration)

The 90% vs 50% split is the key finding. When a scout agent first identified the exact function, file path, and root cause, the builder agent almost always succeeded. When a builder agent was given a vague "implement feature X" prompt, it succeeded about half the time.

This is the scout-constrain-build pattern:

1. **Scout**: An explore agent reads the codebase and identifies exactly what needs to change — file paths, function names, API signatures, test patterns.
2. **Constrain**: The scout's output becomes the builder's input. A vague task is converted into a precise specification.
3. **Build**: A worktree agent implements the change in isolation, runs `cargo fmt && cargo clippy -p <crate> --tests && cargo test -p <crate>`, commits, and creates a PR.

Each agent operates in its own worktree — a complete copy of the repository at a known-good commit. No shared state. No branch conflicts. No file contention. The microcrate architecture from Era 3 means that two agents editing different crates will never produce a merge conflict.

The skill library — 8 skills, 48 commands — encodes the mechanics of recurring operations. `/verify` runs the canonical gate for a crate. `/parser-fix` follows the TDD loop. `/pr-create` handles the GitHub workflow. Agents orchestrate skills rather than reimplementing procedures from scratch.

The memory system — 97 files encoding feedback, project state, and reference pointers — gives agents institutional knowledge that persists across conversations. An agent starting work today can read that "agents adding tests must run `update-current-status.py`" without discovering it through a failed CI run.

The directory lineage captures the transition. `.claude/agents5/` and `.claude/agents6/` record the evolution from the older phase-pack model into persistent swarm teammates like `swarm-scout`, `swarm-builder`, `swarm-reviewer`, and `swarm-strategist`. The current swarm then lives across `.claude/agents/` (effectively the `agents7` layer), `.claude/commands/`, `.claude/skills/`, and `.claude/hooks/`, with `.claude/swarm-state/` acting as the current-ish state and documentation layer around it.

**What Era 5 left behind**: 44 `worktree-agent-*` branches. A skill library. A memory system. The scout-constrain-build pattern. And a CPAN corpus parse rate of 80% — the threshold for the 0.12.0 public alpha.

---

## The Velocity Paradox

| Era | Operating Pace | Quality Signal |
|-----|----------------|----------------|
| 1. Opus Direct | 22.5 commits/active day | Every commit reviewed in conversation |
| 2. Early Swarms | 15.3 commits/active day | PR review, some conflicts |
| 3. Architectural | 8.9 commits/active day | ADRs, mutation testing, formal gates |
| 4. Copilot Fleet | 36.4 merged commits/day | CI pass rate, high duplication |
| 5. Agent Teams | Session-based mixed-tool bursts | Scout-verified, 90% constrained success |

Era 5 does not map cleanly onto the same daily-velocity axis. The swarm only ran a couple of times across the March 11–19 window, and the same period also includes Codex CLI PR waves plus Claude-led cleanup and merge work. What matters is the *structure* of the output.

Era 4 produced 255 merged commits in 7 days and left behind 431 branches to triage. Era 5 left behind 56 reviewed PRs with a clear merge order, 80+ filed issues with line-number references, and a corpus improvement of 8 percentage points while also absorbing and cleaning up waves of Codex CLI output.

The difference is not speed. It is *legibility*. Era 4's output required significant human effort to triage — clustering duplicate PRs, comparing competing solutions, closing the inferior ones. Era 5's output was pre-triaged by the system itself. Scouts identified work. Builders executed it. Reviewers verified it. The human operator directed strategy, not housekeeping.

"Faster" is the wrong metric. The right metric is: *how much human attention does each commit require?* Era 1 required a human for every commit. Era 4 required a human to sort through the mess afterward. Era 5 requires a human to set direction and approve merges.

---

## Branch Naming as Archaeological Signal

The git branch namespace is an unintentional archaeological record. Each naming convention reveals the tool that created it and the methodology behind it:

**Natural names** (Eras 1-2): `add-syntax-highlighting-test`, `backup-before-reset-20251227`. A human chose these names. They describe intent. They are meant to be read.

**`codex/*` with suffixes** (Era 4): `codex/improve-fuzzing-coverage-0zvczn`. The `codex/` prefix identifies the tool. The random suffix (`-0zvczn`) is a collision-avoidance mechanism — evidence that multiple agents received the same task and the system appended entropy to prevent branch name conflicts.

**`worktree-agent-HASH`** (Era 5): `worktree-agent-a0a94df1`. No human-readable component at all. The branch is an execution artifact. Its purpose is recorded in the PR, not the branch name. The hash is a worktree identifier, not a description.

The progression — descriptive to semi-automated to fully mechanical — mirrors the shift from human-in-the-loop to human-on-the-loop to human-at-the-helm.

---

## What Each Era Left Behind

Walk the repository today and you can identify artifacts from every era:

**Era 1** built the bones. The `crates/perl-parser/src/` directory structure, the core AST types in `crates/perl-ast/`, the lexer in `crates/perl-lexer/` — these were designed in conversation and have not been replaced, only extended.

**Era 2** established the workflow. The PR-based development model, the test corpus in `test_corpus/`, the first CI pipeline — all products of the transition from single-threaded to multi-threaded development.

**Era 3** created the safety net. The 7 ADRs in `docs/adr/`, the mutation testing targets, the `just ci-gate` command, the CPAN corpus verification pipeline, the Nix flake. Every quality gate that keeps Era 5's 100 agents from breaking the build was designed during Era 3's deliberate slowdown.

**Era 4** left the crate structure. Despite the duplication and noise, Era 4's SRP microcrate splits — 40 branches attempting the same decomposition, with the best solutions merged — produced the 130-crate architecture that makes Era 5's worktree isolation possible.

**Era 5** is leaving the institutional memory. The skills, the commands, the memory files, the scout-constrain-build methodology — these are not code artifacts but *process* artifacts. They encode how to work on this codebase, not just what the codebase contains.

---

## The Key Insight

The five eras converge on a single principle: **understanding before acting, constraints before freedom**.

Era 1 understood deeply but acted slowly. Era 4 acted fast but understood nothing. Era 5 found the synthesis: scout first, constrain the task, then build within those constraints.

The scout-constrain-build pattern is not specific to AI development. It is the engineering method applied to AI-scale parallelism:

1. **Scout**: Understand the problem. Read the code. Identify the root cause. Name the files, functions, and line numbers.
2. **Constrain**: Convert understanding into specification. A vague task becomes a precise one. "Fix the parser" becomes "modify `parse_postfix_expr` in `crates/perl-parser-core/src/engine/parser/expressions/postfix.rs` to handle the case where a parenthesized argument list follows a named unary operator."
3. **Build**: Execute within the constraints. The builder does not need to understand the whole system. It needs to understand its constraint and verify its output.

The 90% success rate on constrained tasks versus the 50% success rate on unconstrained tasks is the empirical proof. The cost of a 10-minute scout that identifies exact function names, file paths, and test patterns is trivially small compared to the cost of a builder agent that compiles incorrectly, produces the wrong abstraction, or duplicates existing work.

Research and planning are how you convert unconstrained work into constrained work. Every minute of scouting reduces builder failure rate. The optimal ratio appears to be 3:1 for novel features (three scouts per builder) and 1:3 for well-understood patterns (one scout per three builders).

The project's trajectory — from 947 commits of deep, considered work to session-based, structured parallel work in March 2026 — is not a story about AI getting faster. It is a story about a development methodology getting smarter. The code was always cheap. The trusted change was always the hard part.

---

*perl-lsp is an open-source Perl Language Server. The git history referenced in this article is publicly available. All commit counts, branch names, and dates are drawn directly from `git log`.*

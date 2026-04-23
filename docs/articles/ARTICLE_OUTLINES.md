# perl-lsp Launch Articles: Structured Outlines

Eight publication-ready article outlines for the perl-lsp 0.12.0 public alpha launch. Each outline combines existing article material with interview questions to produce compelling, evidence-backed narratives.

---

## Article 1: "100 Agents, 56 PRs, 5 Days"

**Subtitle**: What happens when you point a hundred AI agents at a Rust codebase and try not to break everything

**Target audience**: Engineering leaders, VP/Director-level, CTOs evaluating AI-assisted development at scale

**Key thesis**: Scaling AI code generation is trivial; scaling *trusted change* requires methodology, architecture, and the discipline to slow down before speeding up.

### Section outline

1. **The Promise and the Firehose**
   - Era 4's 152-commit day: what raw AI throughput looks like
   - 431 branches, 40 duplicate PRs for one task, CI queue meltdown
   - The realization: "volume without structure is noise"

2. **What "Trusted Change" Actually Means**
   - Code is cheap; review, testing, CI, and merge are expensive
   - DevLT (Developer Lead Time) as the metric that matters
   - Traditional bottleneck: one senior developer reviewing serially

3. **The Architecture That Enables Parallelism**
   - 133 crates, zero circular dependencies
   - Microcrate architecture as the unit of safe parallel work
   - Git worktree isolation: no shared mutable state between agents

4. **Scout-Constrain-Build: The Pattern That Changed Everything**
   - 90% success on constrained tasks vs 50% unconstrained
   - A 10-minute scout prevents a 60-minute failed build
   - The optimal ratio: 3 scouts per builder for novel work, 1 per 3 for patterns

5. **The Five Coordinator Model**
   - Scout, Builder, Reviewer, Ops, Improver
   - 5 coordination slots managing 20-100 parallel workers
   - Pull-based pipeline: merge agents pull green PRs, not push

6. **What 56 PRs in 5 Days Actually Produced**
   - CPAN corpus parse rate: 72% to 80% (345 new clean files)
   - 80+ issues filed with file-path and line-number references
   - Parser fixes, LSP features, VSCode extension, docs, infrastructure

7. **The Real Bottleneck**
   - Not agent capacity: CI throughput and merge queue width
   - 9 concurrent builders is optimal (merge_queue_width x agent_work_time / merge_cycle_time)
   - Diminishing returns curve: first 15 agents deliver most value

8. **Lessons for Engineering Leaders**
   - Invest in architecture before agents
   - Measure DevLT, not commits
   - The human role shifts from writer to director

### Pull quotes

1. "Four branches for 'improve fuzzing coverage.' Each agent received the same high-level prompt and independently decided what to do. The results were predictable: overlapping work, conflicting approaches, and a merge queue that grew faster than it could be drained."
2. "The 90% vs 50% split is the key finding. When a scout agent first identified the exact function, file path, and root cause, the builder agent almost always succeeded."
3. "Era 4 produced 255 merged commits in 7 days and left behind 431 branches to triage. Era 5 left behind 56 reviewed PRs with a clear merge order."

### Interview questions to enhance

- **Q13**: "You've run sessions with 100 agents. What does that feel like? What's the human experience?" -- Opens the emotional/experiential dimension that data alone cannot convey
- **Q8**: "The scout-constrain-build pattern achieves 90% success on constrained tasks vs 50% unconstrained. How did you discover that 2x difference?" -- Grounds the methodology in empirical evidence
- **Q15**: "What's the hardest lesson the swarm taught you?" -- Provides the honest reflection that makes the article credible

### Word count estimate

3,500-4,500 words

### Source documents

- `FIVE_ERAS.md` (Eras 4-5, Velocity Paradox section)
- `SWARM_METHODOLOGY.md` (Sections 3-6, 10-11)
- `CURIOSITIES.md` (Swarm in Numbers, Records and Extremes)
- `interview-questions.md` (Q5, Q8, Q13-15)

---

## Article 2: "Only Rust Can Parse Perl"

**Subtitle**: How a hand-written recursive descent parser in Rust handles the language that famously cannot be parsed

**Target audience**: Language tooling developers, compiler engineers, PL enthusiasts

**Key thesis**: Perl's context-sensitive grammar defeats every parser generator; only a hand-written recursive descent parser with a stateful lexer can handle the 10 fundamental ambiguities -- and even then, 100% coverage is provably impossible.

### Section outline

1. **Larry Wall Was Right (Mostly)**
   - "Only perl can parse Perl" -- the famous quote and what it actually means
   - Context-sensitive grammar: lexer needs parser state, parser needs runtime state
   - Source filters: modules that rewrite code before the parser sees it

2. **The 10 Ambiguities**
   - `/` as division or regex (the ambiguity that killed tree-sitter)
   - `{}` as hash ref, block, or bare block
   - Heredocs that break left-to-right parsing
   - Prototypes vs signatures (requires knowing `use feature` state)
   - Special variables that look like operators (`$/`, `$$`, `$^W`)

3. **Three Parsers, One Language**
   - v1: Tree-sitter (LR) -- context-free grammars cannot express Perl
   - v2: Pest (PEG) -- ordered choice and backtracking were not enough
   - v3: Recursive descent -- stateful lexer, contextual parsing, arbitrary lookahead
   - Why the repo is named `tree-sitter-perl-rs` but doesn't use tree-sitter

4. **The Stateful Lexer**
   - `LexerMode` state machine: `ExpectTerm` vs `ExpectOperator`
   - How context tracking solves the `/` ambiguity at the token level
   - Dedicated modes: `InFormatBody`, heredoc collection, regex delimiters

5. **The CPAN Corpus Oracle**
   - 4,355 real-world Perl files from CPAN
   - Error buckets: 30 categories, each a family of related failures
   - Ratcheting: the parse rate can only go up, never down
   - 80% clean parses, ~275 microseconds per file

6. **What We Cannot Parse (And Why That's Honest)**
   - Source filters (fundamentally incompatible with static analysis)
   - `BEGIN` blocks that modify the parser at compile time
   - Eval'd code, custom DSLs via import (Moose, Moo)
   - The remaining 20%: not "haven't implemented" but "provably requires runtime"

7. **Error Recovery for IDEs**
   - Partial ASTs with ERROR nodes (the parser never stops at the first error)
   - Synchronization to statement boundaries
   - Recursion protection: MAX_RECURSION_DEPTH = 128, lexer budgets

### Pull quotes

1. "The same sequence of characters can mean completely different things depending on what came before, what module has been `use`d, and even what subroutine prototypes are in scope."
2. "Tree-sitter grammars are context-free. Perl's grammar is context-sensitive. The `/` ambiguity alone defeats any context-free approach."
3. "The parser handles the entire 4,355-file CPAN corpus in about 1.2 seconds on a single core. Typical developer files parse in under a millisecond."

### Interview questions to enhance

- **Q2**: "What was the 'aha moment' that led to the recursive descent parser?" -- Captures the pivotal technical decision
- **Q16**: "Larry Wall said 'only Perl can parse Perl.' Do you agree?" -- Frames the philosophical tension at the heart of the project
- **Q17**: "What's the weirdest Perl syntax you've had to handle?" -- Provides the concrete war stories that make technical articles memorable

### Word count estimate

4,000-5,000 words

### Source documents

- `PARSING_PERL.md` (primary source -- Sections 1-7)
- `FIVE_ERAS.md` (three-parser history)
- `CURIOSITIES.md` (Perl-Specific Weirdness, Three Parser Story)
- `interview-questions.md` (Q2-3, Q16-18, Q24-25)

---

## Article 3: "Code Is Cheap; Trusted Change Is Not"

**Subtitle**: A methodology for making AI-generated code trustworthy by construction, not by hope

**Target audience**: AI/ML practitioners, developers using AI coding assistants, engineering managers adopting AI tools

**Key thesis**: The bottleneck in AI-assisted development is not code generation -- it is building trust in the generated code through adversarial review, automated gates, and receipts that prove correctness.

### Section outline

1. **The Attention Bottleneck**
   - Software development doesn't parallelize because review is serial
   - Ten engineers can write ten patches; they queue behind one pair of eyes
   - LLMs made code generation free; trust is what costs time

2. **The Cost Model Nobody Measures**
   - DevLT: minutes of human attention per trusted change
   - Traditional: $150-250/hr senior dev, serial, 3-8 changes/day
   - Swarm: $1-5 per agent flow, parallel, 40-80 changes/session
   - Scaling from adding humans (months) to adding compute (seconds)

3. **Nobody Grades Their Own Homework**
   - Adversarial structure: author writes, critic verifies, reviewer audits
   - The build loop: code -> format -> lint -> test -> draft PR
   - Why removing any critic degrades the trust guarantee

4. **Build Receipts: Artifacts Over Assertions**
   - Agent claims are worthless; receipts are everything
   - What a receipt contains: requirements, tests, verification output, files changed
   - How one human oversees 100 agents: read receipts, spot-check anomalies

5. **Three Failure Modes and Their Defenses**
   - Hallucination: killed by compilation (hallucinated code doesn't build)
   - Reward hacking: killed by mutation testing and adversarial review
   - Process confabulation: killed by receipts (no artifact, no claim)

6. **The Firehose vs The Pipeline**
   - Era 4: 82 commits/day, half needed rework
   - Era 5: 40 commits/day, dramatically more throughput
   - Structure beats volume every time

7. **Metrics That Actually Matter**
   - "If it scales with compute, it isn't measuring progress"
   - Trust throughput, merge success rate, corpus coverage
   - The ratchet: gains are permanent, regressions are blocked

### Pull quotes

1. "Code is cheap. Trusted change is not. Anyone can generate a patch. LLMs have made that nearly free. The expensive part is everything that turns a patch into a change you'd bet production on."
2. "Agent claims are worthless. Receipts are everything. When a builder completes its work, the output is not 'I fixed the bug' -- it is a structured receipt."
3. "A 40-commit day where every commit is trusted beats an 82-commit day where half the commits need rework."

### Interview questions to enhance

- **Q7**: "What does 'DevLT is the scarce resource' mean to you?" -- Explains the origin of the core metric
- **Q14**: "'When receipts lie' -- was that from a real incident in this codebase?" -- Grounds the methodology in a real failure story
- **Q20**: "'Draft first, review before CI triggers' -- why is that order important?" -- Reveals process discipline that readers can apply

### Word count estimate

3,000-4,000 words

### Source documents

- `SWARM_METHODOLOGY.md` (Sections 1-2, 6-8, 10-11)
- `FIVE_ERAS.md` (Code Is Cheap intro, Velocity Paradox)
- `ZERO_PANIC.md` (When Receipts Lie, Mutation Testing)
- `interview-questions.md` (Q7, Q14, Q19-20)

---

## Article 4: "Five Eras of AI Development"

**Subtitle**: How one project passed through solo coding, early swarms, architectural slowdown, AI firehose, and structured agent teams -- and what each era taught about velocity vs progress

**Target audience**: General tech audience, developers interested in AI trends, tech journalists

**Key thesis**: The trajectory of AI-assisted development is not a straight line toward faster -- it is a curve toward better, and the critical insight is that intentional deceleration (Era 3) enabled everything that followed.

### Section outline

1. **The Git History as Archaeological Record**
   - 2,697 commits tell the whole story if you know how to read them
   - Branch naming evolution: human-chosen -> codex/* -> worktree-agent-HASH
   - Each naming convention reveals the tool and the methodology

2. **Era 1: Opus Direct (July-August 2025)**
   - One developer, one AI, one chat window
   - 22.5 commits/active day, every commit understood in conversation
   - Built the foundation: parser, lexer, AST, test infrastructure

3. **Era 2: Early Swarms (August-October 2025)**
   - First experiment with parallelism, commit rate drops to 15.3/day
   - Discovery: crate boundaries are isolation boundaries
   - Breadth over depth: LSP, scope analyzer, DAP work in parallel

4. **Era 3: The Slowdown That Made Everything Possible**
   - 8.9 commits/active day -- the slowest era and the most important
   - 7 ADRs, mutation testing, microcrate extraction, CPAN corpus pipeline
   - "You cannot run 100 agents in parallel on a monolithic codebase. You can on 130 microcrates."

5. **Era 4: The Firehose**
   - 152 commits on a single day; 431 branches; 40 duplicates of one task
   - Peak volume, peak waste: three agents fixing the same bug
   - The lesson: velocity without legibility is noise

6. **Era 5: The Synthesis**
   - Scout-constrain-build: understand before acting, constrain before building
   - 56 reviewed PRs, 80+ filed issues, CPAN 72% to 80%
   - Human role shifts from writing to directing

7. **The Velocity Paradox**
   - Era 4 was fastest by metrics, Era 5 was fastest by outcomes
   - "Faster" is the wrong metric; "how much human attention does each commit require?" is right
   - The progression: human-in-the-loop -> human-on-the-loop -> human-at-the-helm

8. **What Each Era Left Behind**
   - Era 1: the bones (parser architecture)
   - Era 2: the workflow (PR-based development)
   - Era 3: the safety net (ADRs, mutation testing, CI gates)
   - Era 4: the crate structure (despite the noise)
   - Era 5: the institutional memory (skills, commands, methodology)

### Pull quotes

1. "Era 3 was an intentional deceleration. The architecture was designed in browser-based chat sessions -- long-form conversations about how the system should be structured, not what code to write next."
2. "Three builders independently fixing the same bug is not three times as productive. It is one fix and two wasted CI runs."
3. "Era 1 understood deeply but acted slowly. Era 4 acted fast but understood nothing. Era 5 found the synthesis: scout first, constrain the task, then build within those constraints."

### Interview questions to enhance

- **Q4**: "Era 3 was the slowest but you say it enabled all future speed. What happened?" -- The emotional experience of intentional slowdown
- **Q5**: "The Copilot 'firehose' era hit 82 commits/day. What was that like?" -- The visceral experience of peak chaos
- **Q6**: "What made you switch from Copilot to Claude Code?" -- The turning point between Eras 4 and 5

### Word count estimate

4,000-5,500 words

### Source documents

- `FIVE_ERAS.md` (primary source -- entire document)
- `SWARM_METHODOLOGY.md` (Section 10: The Firehose Lesson)
- `CURIOSITIES.md` (Records and Extremes, branch naming)
- `interview-questions.md` (Q4-6, Q34)

---

## Article 5: "No Panics Allowed"

**Subtitle**: How a zero-panic policy, four enforcement layers, and defense-in-depth security make a language server you can trust with your editor

**Target audience**: Systems programmers, Rust developers, security-conscious engineers

**Key thesis**: A language server is the most intimate piece of infrastructure a developer touches -- it runs on every keystroke with filesystem access -- and reliability requires a policy enforced at compile time, not a guideline followed by convention.

### Section outline

1. **The Silent Crash**
   - When an LSP panics, completions vanish, diagnostics go stale, navigation breaks
   - No error dialog -- just silence; the developer thinks the project is misconfigured
   - Crash loops: if a specific file triggers the panic, restart makes it worse

2. **Seven Banned Constructs**
   - `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `abort()`, `dbg!()`
   - Not guidelines: `deny`-level Clippy lints, compile errors across the entire workspace
   - The one exception: `uri.rs`, documented, justified, and since refactored away

3. **Four Enforcement Layers**
   - Layer 1: Clippy lints (compile time -- syntactic violations)
   - Layer 2: CI gates (pre-merge -- catches misconfigured toolchains)
   - Layer 3: Code review (semantic violations Clippy can't see)
   - Layer 4: CLAUDE.md as AI-agent enforcement (policy in every agent's context)

4. **The Error Handling Pattern**
   - `?` operator as primary tool, `ok_or_else()` for Option-to-Result
   - Graceful regex degradation: `Option<Regex>` via `.ok()`
   - Test code uses `unwrap()` deliberately: test failures should be loud

5. **Supply Chain Security**
   - `deny.toml`: 116-line supply chain firewall
   - Source restrictions, license allowlist, advisory monitoring
   - Zero GPL, zero AGPL -- only OSI-approved permissive licenses

6. **Path Traversal Prevention**
   - 3-layer defense: character validation, canonical resolution, non-existent path normalization
   - Belt-and-suspenders: final path checked against workspace boundary twice
   - Not defense-in-depth by design -- each layer added after discovering edge cases in the previous

7. **DAP Expression Safety**
   - The `SafeEvaluator`: 6 validation checks before expressions reach the debugger
   - Context-aware filtering: `$print` is a variable, not the `print` builtin
   - Default safe mode with opt-in `allowSideEffects` for power users

8. **"When Receipts Lie" and Mutation Testing**
   - The benchmark confabulation: technically correct, operationally meaningless
   - Mutation testing as the answer: "if this claim were wrong, would you notice?"
   - Defense in depth: each layer compensates for the blind spots of the others

### Pull quotes

1. "A language server crash is different. The editor spawns the LSP process in the background. When that process panics, the user's experience degrades silently."
2. "A green CI gate proves that the code compiles and the tests pass. It does not prove that the tests test the right thing."
3. "Test output is not evidence of correctness. This is why perl-lsp invests in mutation testing."

### Interview questions to enhance

- **Q10**: "The zero-panic policy -- was there a specific incident that led to it?" -- The origin story of the policy
- **Q14**: "'When receipts lie' -- was that from a real incident?" -- The benchmark confabulation story, grounded and honest

### Word count estimate

3,500-4,500 words

### Source documents

- `ZERO_PANIC.md` (primary source -- entire document)
- `CURIOSITIES.md` (Hidden Infrastructure, the one allowed `expect()`)
- `SWARM_METHODOLOGY.md` (Section 7: Build Receipts)
- `interview-questions.md` (Q10, Q14)

---

## Article 6: "The Self-Improving Swarm"

**Subtitle**: How persistent memory, composable skills, and a feedback loop that spans sessions turn 100 disposable agents into an institution that learns

**Target audience**: AI researchers, AI agent framework developers, autonomous systems designers

**Key thesis**: The innovation is not the agents -- it is the institutional knowledge layer (memory, skills, hooks) that makes the 50th session better than the 1st and turns disposable workers into a compounding system.

### Section outline

1. **The Goldfish Problem**
   - Every AI agent starts from zero: no memory of what worked, what failed, what was tried
   - Prompt instructions are suggestions agents forget under pressure
   - Without institutional knowledge, the 50th session is as expensive as the 1st

2. **Memory as Institutional Knowledge**
   - 98 persistent memory files across 4 types: user, feedback, project, reference
   - Feedback memories: "rebase --ours/--theirs is INVERTED from merge; agents get this wrong systematically"
   - Memory is how lessons from a failed agent become wisdom for future agents

3. **Skills as Composable Procedures**
   - From 52 agent definitions to 3 patterns + 10 skills
   - Skills compound: each new skill makes every future agent prompt shorter
   - Before `/verify-build`: 3 lines of cargo commands per prompt. After: one line.

4. **Hooks as Deterministic Enforcement**
   - Cycle 2: agents told to write metrics entries; zero of 30 PRs had them
   - After hooks: compliance was automatic
   - "Prompt instructions are suggestions. Hooks are enforcement."

5. **The Learning Loop Across Sessions**
   - ~20% capacity always reserved for self-improvement
   - Feedback -> memory -> skills -> hooks -> agent behavior
   - The swarm improves itself as a side effect of doing its work

6. **Context Layering**
   - CLAUDE.md (project-wide) -> hooks (behavioral) -> agent def (role) -> skills (mechanics)
   - Each layer more specific than the last
   - Skills encode HOW; agent defs encode WHAT; memory encodes WHY

7. **The Compounding Effect**
   - Agent success rate improved from ~50% (Era 4) to ~90% (Era 5, constrained)
   - The same task type that failed in Cycle 1 succeeds in Cycle 5
   - Institutional knowledge compounds across every session, every agent

### Pull quotes

1. "52 agent definitions, 3 actual patterns. The later swarm kept the agent layer but pushed mechanical step instructions into composable skills."
2. "In cycle 2, agents were told to write metrics entries. Zero of 30 PRs had metrics. After hooks were added to enforce the requirement, compliance was automatic."
3. "Memory is the mechanism that makes the 50th session better than the 1st. When a new agent encounters a parser error bucket, it doesn't start from scratch."

### Interview questions to enhance

- **Q28**: "Is the swarm methodology the product, or is perl-lsp the product?" -- Reveals whether the learning system is the real innovation
- **Q23**: "What's the most important thing the project has taught you that you didn't expect to learn?" -- Captures surprising insights about AI self-improvement
- **Q19**: "The scout-constrain-build pattern -- is it specific to this project, or a general principle?" -- Tests generalizability

### Word count estimate

3,000-4,000 words

### Source documents

- `SWARM_METHODOLOGY.md` (Sections 5, 9: Skills, Memory, Hooks)
- `FIVE_ERAS.md` (Era 5 skill library, memory system)
- `CURIOSITIES.md` (Swarm in Numbers, 52 agent definitions -> 3 patterns)
- `interview-questions.md` (Q19, Q23, Q28)

---

## Article 7: "130 Crates, Zero Conflicts"

**Subtitle**: How microcrate architecture enables 100 parallel AI agents to work on the same Rust project without a single merge conflict

**Target audience**: Rust community, systems architects, large-codebase maintainers

**Key thesis**: The granularity of your module decomposition determines the parallelism ceiling for both human and AI contributors -- and Rust's workspace model makes extreme decomposition practical.

### Section outline

1. **The Parallelism Problem**
   - Two agents editing the same file always conflict
   - Traditional Rust projects: 3-5 crates, merge conflicts at scale
   - The question: what is the unit of safe parallel work?

2. **The Microcrate Architecture**
   - 133 crates organized into families: `perl-module-*`, `perl-lsp-*`, `perl-dap-*`
   - Each crate: single responsibility, narrow public API, independently testable
   - Smallest crate: `perl-line-index` (44 lines); largest: `perl-lsp` (120,596 lines)

3. **How It Happened**
   - Era 3's deliberate slowdown: 54 days of architectural extraction
   - Era 4's accidental validation: 40 agents splitting god files, best solutions merged
   - ADR-0008: the formal decision and its rationale

4. **Zero Circular Dependencies**
   - 133 crates, zero cycles in the dependency graph
   - Family organization prevents accidental coupling
   - Rust's type system enforces boundaries at compile time

5. **The Parallelism Payoff**
   - 50-100 agents working simultaneously with zero file conflicts
   - Each agent in its own git worktree, touching a different crate
   - Architecture IS the parallelism enabler

6. **Tradeoffs and Tooling Limits**
   - `cargo build` is fast per-crate but workspace-wide compilation is slower
   - IDE integration: rust-analyzer handles 133 crates but needs memory
   - Dependency management: `cargo machete` for unused deps, `semver-check` for API stability

7. **Feature Governance**
   - `features.toml`: 116 LSP/DAP capabilities governed through a 3-stage pipeline (87 LSP + 24 DAP + 5 extension, post PR #4107 audit)
   - Feature microcrates (`perl-lsp-feature-*`) gate capability advertisement
   - Dual indexing: symbols indexed under both qualified and bare names

### Pull quotes

1. "You cannot run 100 agents in parallel on a monolithic codebase. You can on 130 microcrates with formalized boundaries."
2. "133 crates, zero circular dependencies. The microcrate architecture makes this possible: each crate has a single responsibility and a narrow public API."
3. "The smallest meaningful crate is `perl-line-index` at 44 lines. Tiny, correct, and depended on by dozens of other crates."

### Interview questions to enhance

- **Q9**: "130 crates for a language server seems extreme. How did that happen? Was it planned?" -- Captures whether this was intentional or emergent
- **Q11**: "Why dual indexing (qualified + bare names)?" -- A Perl-specific design choice that reveals language-informed architecture
- **Q12**: "Why feature governance with 7 microcrates instead of just shipping everything?" -- Shows the depth of the decomposition philosophy

### Word count estimate

3,000-3,500 words

### Source documents

- `FIVE_ERAS.md` (Era 3 microcrate extraction, Era 4 god file splits)
- `CURIOSITIES.md` (Architecture Curiosities, smallest/largest crate)
- `SWARM_METHODOLOGY.md` (Section 9: Worktree Isolation)
- `interview-questions.md` (Q9, Q11-12)

---

## Article 8: "From CPA to LSP"

**Subtitle**: How an accounting background shaped a language server project -- and what receipts, ledgers, and audit trails teach about building software with AI

**Target audience**: Non-traditional developers, career changers, general tech audience

**Key thesis**: The accounting mindset -- accuracy, audit trails, reconciliation, and skepticism of self-reported numbers -- turns out to be exactly the right mental model for overseeing AI agents that confabulate process compliance.

### Section outline

1. **An Unlikely Builder**
   - CPA building a Perl language server in Rust with 100 AI agents
   - The intersection of accounting rigor and software engineering
   - Why Perl? Why now? What was missing in the Perl developer experience?

2. **The Accounting Metaphors Are Not Accidental**
   - DevLT as a "budget model" for human attention
   - "Tech debt ledger" (`.ci/debt-ledger.yaml`) -- literal ledger in the codebase
   - "Receipts" -- the word accountants use for evidence, applied to AI output

3. **Receipts Over Self-Reported Claims**
   - In accounting: never trust a number without the source document
   - In AI: never trust an agent's assertion without the build artifact
   - "When receipts lie" -- the benchmark confabulation story

4. **The Ratchet as Reconciliation**
   - Accountants reconcile: does the balance match the ledger?
   - CPAN corpus ratchet: the parse rate can only go up, never down
   - CI as continuous audit: every PR is examined before it enters the books

5. **Five Eras as a Learning Journey**
   - From solo conversations (Era 1) to managing 100 agents (Era 5)
   - Each era taught a lesson about what doesn't parallelize
   - The slowdown (Era 3) as the most accountant-like decision: invest before spending

6. **What Accounting Teaches Software**
   - Double-entry: every change has a test (the balancing entry)
   - Audit trail: git history is the general ledger
   - Materiality: not every bug is worth fixing; focus on what moves the needle
   - Skepticism of round numbers and self-assessment

7. **The Personal Journey**
   - What makes someone leave accounting for language tooling?
   - The role of AI as equalizer: domain knowledge + AI capability > traditional credentials
   - What got you through the hardest moments?

### Pull quotes

1. "Code is cheap. Trusted change is not. Anyone can generate a patch. The expensive part is everything that turns a patch into a change you'd bet production on."
2. "An agent once wrote a benchmark that was technically correct, operationally meaningless. The benchmark compiled. The tests passed. The CI gate was green. The numbers looked plausible. The problem: it was measuring the wrong thing."
3. "If it scales with compute, it isn't measuring progress."

### Interview questions to enhance

- **Q21**: "You're a CPA, not a traditional developer. How does that background shape how you think about this project?" -- The core question of the article
- **Q22**: "Did you ever want to give up on this project?" -- Vulnerability and honesty
- **Q35**: "What's the one thing about Perl that surprised you most?" -- Fresh perspective from an outsider-become-insider

### Word count estimate

2,500-3,500 words

### Source documents

- `FIVE_ERAS.md` (opening section, velocity paradox)
- `SWARM_METHODOLOGY.md` (Sections 2, 7, 11: cost model, receipts, metrics)
- `ZERO_PANIC.md` (When Receipts Lie)
- `CURIOSITIES.md` (By the Numbers, project start date)
- `interview-questions.md` (Q1, Q21-23, Q33-35)

---

## Cross-Article Reference Matrix

| Interview Question | Article 1 | Article 2 | Article 3 | Article 4 | Article 5 | Article 6 | Article 7 | Article 8 |
|--------------------|-----------|-----------|-----------|-----------|-----------|-----------|-----------|-----------|
| Q1 (Why Perl?) | | | | | | | | primary |
| Q2 (Aha moment) | | primary | | | | | | |
| Q4 (Era 3 slowdown) | | | | primary | | | | |
| Q5 (Firehose era) | secondary | | | primary | | | | |
| Q6 (Copilot -> Claude) | | | | primary | | | | |
| Q7 (DevLT) | secondary | | primary | | | | | |
| Q8 (90% vs 50%) | primary | | secondary | | | | | |
| Q9 (130 crates) | | | | | | | primary | |
| Q10 (Zero-panic origin) | | | | | primary | | | |
| Q13 (100 agents) | primary | | | | | | | |
| Q14 (When receipts lie) | | | primary | | secondary | | | secondary |
| Q15 (Hardest lesson) | primary | | | | | | | |
| Q16 (Only perl parses Perl) | | primary | | | | | | |
| Q17 (Weirdest syntax) | | primary | | | | | | |
| Q19 (Scout-constrain-build) | | | | | | primary | | |
| Q21 (CPA background) | | | | | | | | primary |
| Q22 (Give up?) | | | | | | | | primary |
| Q23 (Unexpected learning) | | | | | | primary | | |
| Q28 (Swarm as product) | | | | | | primary | | |
| Q35 (Perl surprise) | | secondary | | | | | | primary |

---

## Publication Sequencing

**Recommended order for maximum impact:**

1. **"Five Eras of AI Development"** -- Sets the narrative arc, accessible to broadest audience
2. **"100 Agents, 56 PRs, 5 Days"** -- The scale story, builds on era framework
3. **"Code Is Cheap; Trusted Change Is Not"** -- The methodology deep dive
4. **"Only Rust Can Parse Perl"** -- The technical challenge story
5. **"From CPA to LSP"** -- The human story, provides emotional anchor
6. **"No Panics Allowed"** -- Technical depth for Rust community
7. **"130 Crates, Zero Conflicts"** -- Architecture story for Rust community
8. **"The Self-Improving Swarm"** -- AI research angle, most forward-looking

**Pair well together:**
- Articles 1 + 3 (scale + methodology) for engineering leadership publications
- Articles 2 + 7 (parsing + architecture) for Rust/PL community
- Articles 4 + 5 (eras + personal story) for general tech publications
- Articles 6 + 8 (self-improvement + reliability) for AI research venues

---

*Generated from source material in `docs/articles/` and `.claude/interview-questions.md`. Each outline is designed to be handed to a writer with the source documents for full article development.*

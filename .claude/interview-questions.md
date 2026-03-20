# perl-lsp: Interview Questions for Launch Articles

These are compelling questions TO ASK Steven Zimmerman about the perl-lsp project's development history, unique architecture, and insights into AI-assisted development. Each question includes evidence from the codebase that contextualizes the answer.

---

## ORIGIN STORY & MOTIVATION

### Q1: Why Perl? Why now?
**The Question**: Perl seems like an unusual choice for a major LSP project in 2025. Most language servers for older languages were built 10+ years ago. What prompted you to build a new Perl LSP in 2024-2025?

**Why it's interesting**: The repo started in July 2025, relatively recently. Understanding the timing reveals what changed in the Perl ecosystem and the developer experience that made this work feel urgent.

**Evidence from codebase**:
- README states "Perl developers often end up stitching together separate parser, editor, and debugger stories"
- Project integrates LSP, DAP, parser, and lexer — addressing a fragmented ecosystem
- CPAN corpus pipeline suggests a focus on real-world Perl code coverage

**Follow-up if answer goes deep**:
- How many Perl developers were you trying to reach?
- What was the Perl developer experience like before this project?
- Are there Perl use cases that would have been impossible to support without this?

---

### Q2: What was the "aha moment" that led to the recursive descent parser?
**The Question**: The repository uses a v3 recursive descent parser, not tree-sitter. What was the realization that made you abandon tree-sitter and build a custom parser from scratch?

**Why it's interesting**: This is a major architectural decision. The repo is named `tree-sitter-perl-rs`, yet the tree-sitter parser was abandoned. Understanding this pivot reveals what made custom parsing necessary.

**Evidence from codebase**:
- Multiple references to "v3 recursive descent parser" as current approach
- ADR-0001 discusses "substitution operator parsing architecture" — suggests pattern-matching complexity drove the decision
- Git history shows parser being rewritten multiple times (8+ major approaches)
- The tree-sitter implementation exists but is marked as legacy/out-of-default-gate

**Follow-up if answer is technical**:
- What specific Perl patterns broke tree-sitter's ambiguity handling?
- Did you try PEST before going full recursive descent?
- How long did the tree-sitter experiment last?

---

### Q3: The repo name is tree-sitter-perl-rs — but tree-sitter was abandoned. What's the story?
**The Question**: Why keep the tree-sitter name in the repository if the tree-sitter approach was abandoned? Was this a naming artifact, or was there a deliberate choice to anchor the name to a legacy approach?

**Why it's interesting**: Names encode history. Understanding why the repo kept the tree-sitter name reveals whether this was opportunistic naming, a pivot from an earlier effort, or intentional anchoring to tooling that proved inadequate.

**Evidence from codebase**:
- `crates/tree-sitter-perl-c/` exists but is marked with "needs libclang" note — suggests a C-based tree-sitter approach was attempted
- `tree-sitter-perl/` directory exists separately as "legacy C" (separate from main repo)
- CLAUDE.md marks both as workspace exclusions
- v3 parser is the default, v2 (PEST) kept out of default gate

**Follow-up**:
- Did you own the tree-sitter-perl project before pivoting?
- How much of the tree-sitter scaffolding remains?
- What would have to change for you to reconsider tree-sitter?

---

## THE FIVE ERAS: VELOCITIES AND TURNING POINTS

### Q4: Era 3 was the slowest (8.9 commits/day) but you say it enabled all future speed. What happened during that period?
**The Question**: The "Architectural Sidechain" era from October 2025 to February 2026 was the slowest by every metric. Git shows 478 commits across 54 active days. Why intentionally slow down, and what changed that made the slowness worthwhile?

**Why it's interesting**: This inverts the common startup narrative of "go fast and break things." The project instead went slow to build for speed. Understanding this mindset reveals how you think about technical debt and long-term velocity.

**Evidence from codebase**:
- FIVE_ERAS.md states: "An intentional deceleration. The architecture was designed in browser-based chat sessions — long-form conversations about how the system should be structured"
- Seven ADRs were written during this period (docs/adr/0001 through 0007)
- Mutation testing infrastructure was hardened across critical crates
- Microcrate architecture extraction split monolithic crates into single-responsibility modules
- 130 crate directories exist today (result of this period's decomposition)

**Follow-up questions**:
- Did you feel the slowdown at the time? What was the emotional experience?
- When did you first see payoff from the architectural work?
- Was there pressure to ship faster during this period?

---

### Q5: The Copilot "firehose" era hit 82 commits/day. What was that like? When did you realize volume wasn't the answer?
**The Question**: Era 4 (February 28 to March 5, 2026) was a chaotic peak: 152 commits on March 4 alone, 431 remote branches, massive duplication. What was the experience of running that experiment, and what made you pivot away from it?

**Why it's interesting**: This is a cautionary tale about AI-driven development. Volume without structure is chaos. The question reveals whether you discovered this through pain or intuition.

**Evidence from codebase**:
- FIVE_ERAS.md: "Four branches for 'improve fuzzing coverage.' Each agent received the same high-level prompt and independently decided what to do. The results were predictable: overlapping work, conflicting approaches, and a merge queue that grew faster than it could be drained."
- Branch naming shows collision avoidance: `codex/improve-fuzzing-coverage`, `codex/improve-fuzzing-coverage-0zvczn`, `codex/improve-fuzzing-coverage-5o0n3s`, `codex/improve-fuzzing-coverage-g44l28`
- The firehose produced 40 branches for the same "split god files" task
- Memory notes: "40 agents independently attempting to split the same god files into microcrates. Most produced similar solutions. A few produced better ones. All of them consumed CI time."

**Follow-up if answer is personal**:
- Did you enjoy the chaos, or was it stressful?
- What was the moment you realized the volume approach was unsustainable?
- Did any of the duplicated work turn out to be genuinely valuable?

---

### Q6: What made you switch from Copilot to Claude Code? What capability was the tipping point?
**The Question**: Era 4 used Copilot CLI fleet mode. Era 5 switched to Claude Code agent teams. This is a significant pivot in tooling. What did Claude Code enable that Copilot didn't?

**Why it's interesting**: Tooling choices reveal assumptions about how AI should be deployed. The pivot suggests Claude Code unlocked a new capability or way of thinking.

**Evidence from codebase**:
- FIVE_ERAS.md marks March 11 as the shift: "Claude Code Agent Teams (March 11 -- March 19, 2026)"
- Era 5 brought 5 coordinator teams, scout-constrain-build pattern, persistent team structure
- Memory notes: "Scout-constrain-build gets 90% success vs 50% unconstrained"
- CLAUDE.md documents the orchestrator model and skill library
- Team structure: scout, builder, reviewer, ops, improver — highly structured roles
- `.claude/agents6/` vs `.claude/agents7/` show evolution of agent definitions

**Follow-up**:
- Was it the team model, the memory system, the persistent coordinators, or something else?
- What did Copilot CLI lack that you needed?
- Are you still using any Copilot-based tooling, or did you migrate fully?

---

## THE FIVE ERAS: EVIDENCE AND METRICS

### Q7: What does "DevLT is the scarce resource" mean to you? How did that shift your thinking?
**The Question**: The codebase includes a concept called "DevLT" (Developer Latency Time) — the human attention minutes spent on a change. This is not a common metric. What prompted you to invent it, and how did it change how you work?

**Why it's interesting**: Metrics reveal what you optimized for. Most projects optimize for throughput. This one optimizes for human attention cost.

**Evidence from codebase**:
- AGENTIC_DEV.md defines DevLT explicitly: "Human attention minutes spent on a change. Includes: Reading and understanding, decision-making, reviewing output, fixing problems, waiting for feedback loops."
- Bands: Quick (<30 min), Standard (30-120 min), Complex (120+ min)
- "DevLT is the scarce resource. Minimize it."
- Contrasts with Compute Cost (tokens, CI minutes) — frames compute as a lever, not a rival

**Follow-up questions**:
- How do you measure DevLT in practice?
- Have you found a ratio of compute-to-DevLT that felt optimal?
- What decision did DevLT metric change?

---

### Q8: The scout-constrain-build pattern achieves 90% success on constrained tasks vs 50% unconstrained. How did you discover that 2x difference?
**The Question**: Era 5 discovered this empirical split: 90% success for well-specified parser fixes, 50% success for vague feature requests. This is a precise finding. How did you measure this, and what does it tell you about AI-assisted development?

**Why it's interesting**: This is a data-driven insight that inverts common assumptions. It says scouting is critical. The 90% vs 50% split is powerful because it's quantified.

**Evidence from codebase**:
- FIVE_ERAS.md: "The 90% vs 50% split is the key finding. When a scout agent first identified the exact function, file path, and root cause, the builder agent almost always succeeded. When a builder agent was given a vague 'implement feature X' prompt, it succeeded about half the time."
- Memory note: "feedback_agent_success_rate_pattern.md — Constrained tasks ~90% success, unconstrained features ~50%. Break features into constraint-shaped slices."
- The scout pattern is codified in swarm.md under Phase 2: "Scout: An explore agent reads the codebase and identifies exactly what needs to change — file paths, function names, API signatures, test patterns."

**Follow-up**:
- Did you measure failures, or infer from PR merge rate?
- What percentage of success is real (merged + working) vs syntactically correct?
- Does the 90/50 split hold for non-parser work, or is it parser-specific?

---

## ARCHITECTURE DECISIONS

### Q9: 130 crates for a language server seems extreme. How did that happen? Was it planned?
**The Question**: The workspace has 132 crates (counting workspace members). Most language servers are 3-5 crates. This is not incidental. Was this always the plan, or did it grow organically?

**Why it's interesting**: The granularity reflects a philosophical choice about how to structure work. It also affects parallelism, testing, and collaboration.

**Evidence from codebase**:
- ADR-0008 explicitly documents the microcrate architecture decision
- Organized into families: `perl-module-*` (~13), `perl-lsp-*` (~21), `perl-lsp-feature-*` (~7), `perl-dap-*` (~4), `perl-ts-*` (~5), `perl-workspace-*` (~4), ~30 core leaf crates
- ADR context: "Traditional Rust projects typically use fewer, larger crates with internal module separation. This workspace takes a different approach."
- Era 3 left behind "130 crate directories" after intentional SRP decomposition
- Era 4 generated "40 branches for the same 'split god files' task" — suggesting earlier god files that needed decomposition

**Follow-up**:
- At what crate count did you realize this approach was necessary for your parallelism?
- What's the smallest meaningful crate in your system? What's the largest?
- Have you hit any tooling limits with 130 crates?

---

### Q10: The zero-panic policy — was there a specific incident that led to it?
**The Question**: ADR-0012 bans `unwrap()`, `expect()`, `panic!()` in production code. This is strict for a Rust project. What happened that made this policy necessary?

**Why it's interesting**: Incident-driven policies reveal real problems. Understanding the trigger reveals what you learned through pain.

**Evidence from codebase**:
- ADR-0012 is titled "Error Handling Strategy (No Panics Policy)"
- Context: "The Perl LSP server is a long-running process that editors depend on for code intelligence. Unlike command-line tools that can exit on error, an LSP server must remain operational."
- Lists failure modes: Parse Error, I/O Error, Protocol Error, Logic Error, Resource Error
- Banned constructs: unwrap, expect, panic, todo, unimplemented

**Follow-up**:
- Did you hit a production crash in the early LSP days?
- How strict is enforcement? (Grep for violations? CI gate?)
- Are there cases where you allow panics?

---

### Q11: Why dual indexing (qualified + bare names)?
**The Question**: ADR-0009 documents dual indexing where functions are indexed under both their qualified name (`Package::function`) and bare name (`function`). Why is this necessary, and what problem does it solve?

**Why it's interesting**: This is a Perl-specific design choice. Understanding it reveals how the LSP navigates Perl's packaging and context-awareness.

**Evidence from codebase**:
- ADR-0009 explicitly documents the decision
- Problem: "Perl, function references can appear in two forms: Bare name (`function()`) and Qualified name (`Package::function()`)"
- Decision: "Index under both qualified name (`Package::function`) and bare name (`function`)"
- Coverage metrics: 98% of function references discoverable
- Rationale: "70% of Perl code uses bare names" (rejected qualified-only indexing)
- Index size overhead: ~40% increase in index entries

**Follow-up**:
- How do you handle ambiguity? (Multiple packages define the same bare function name)
- Is dual indexing used for variables and other symbols, or just functions?
- What's your conflict resolution strategy?

---

### Q12: Why feature governance with 7 microcrates instead of just shipping everything?
**The Question**: The codebase uses `perl-lsp-feature-*` microcrates for feature governance. Most LSPs just ship all features together. Why decompose the feature set?

**Why it's interesting**: This is a design choice about how to ship features, defaults, and capability discovery. It suggests thinking about Perl developer diversity.

**Evidence from codebase**:
- ADR-0016 documents "Feature Governance"
- 7 `perl-lsp-feature-*` crates exist
- features.toml is the canonical LSP capability definition
- CLAUDE.md mentions feature governance with 7 microcrates

**Follow-up**:
- What are the 7 features you gated? (And are there more now?)
- How do Perl developers control which features to enable?
- Did feature toggles ever save you from shipping a breaking change?

---

## THE SWARM EXPERIENCE

### Q13: You've run sessions with 100 agents. What does that feel like? What's the human experience?
**The Question**: The memory notes reference a "100-agent session" in Cycle 5. What was the human experience of orchestrating 100 parallel agents? Did it feel chaotic, or surprisingly coherent?

**Why it's interesting**: This is a rare scale of AI parallelism. The answer reveals how human attention shifts when you have that many workers.

**Evidence from codebase**:
- Memory: "feedback_100_agent_session.md — 100-agent session learnings: research→build wave pattern, CI bottleneck, status update trap"
- Cycle 5 claimed: "80+ issues filed with file-path and line-number references"
- Memory notes: "team_roster_hard_ceiling ~75. Plan budget upfront. Issues as overflow queue."
- FIVE_ERAS.md: Era 5 had "56 PRs created spanning parser fixes, LSP features, VSCode extension, documentation, and infrastructure"

**Follow-up if answer is personal**:
- What was the bottleneck? (CI queue? Human merge capacity? Decision-making latency?)
- Did you feel you lost visibility at 100 agents, or did structure maintain coherence?
- Would you do 100 again?

---

### Q14: "When receipts lie" — was that from a real incident in this codebase?
**The Question**: There's a talk slide or incident note titled "when receipts lie." This sounds like a specific incident where test output, CI results, or metrics misled. What happened?

**Why it's interesting**: This reveals what assumptions broke. It likely led to process changes.

**Evidence from codebase**:
- AGENTIC_DEV.md discusses "receipt-based claims" vs "trust-based claims"
- Memory note: "feedback_status_update_trap.md — Agents adding tests must run update-current-status.py or policy_checks gate fails"
- LESSONS.md (referenced but not located) would log wrongness incidents
- The phrase "receipts" appears in context of test output, gate output, CI results proving claims

**Follow-up**:
- What metric or test result lied?
- Did you catch it through automation or manual review?
- How did you change the gate?

---

### Q15: What's the hardest lesson the swarm taught you?
**The Question**: After running swarms of 10, 40, and 100 agents across five eras, what single lesson hurt the most to learn and changed your approach fundamentally?

**Why it's interesting**: This is a reflection question that reveals learning arc. It's likely to be more honest than pre-prepared talking points.

**Evidence from codebase**:
- Memory file: "feedback_cycle5_learnings.md — 10 meta-learnings from 75-agent session"
- Learning topics include: dedup waste, ratchet gaps, team ceiling, version drift, policy_checks friction, phantom buckets
- Memory file: "feedback_rebase_semantics_trap.md — Rebase --ours/--theirs is INVERTED from merge; agents get this wrong systematically"
- Memory note about discovered duplication patterns in Era 4

**No direct quote to cite, but the fact that you have a dedicated learnings file suggests you're tracking this.**

---

## PERL-SPECIFIC CHALLENGES

### Q16: Larry Wall said "only Perl can parse Perl." Do you agree?
**The Question**: The codebase quotes Larry Wall's famous maxim. Your answer to this question reveals your core assumptions about what's possible with static analysis of Perl.

**Why it's interesting**: If you agree, your entire LSP is a statement about finding practical boundaries. If you disagree, it's a claim about what's achievable.

**Evidence from codebase**:
- PARSING_PERL.md directly quotes: "Larry Wall once said 'only perl can parse Perl.' He was not exaggerating. Perl's grammar is context-sensitive, ambiguous, and extensible at parse time."
- Same file lists complexity sources: "The same sequence of characters can mean completely different things depending on what came before, what module has been `use`d, and even what subroutine prototypes are in scope."

**Follow-up if thoughtful**:
- What percentage of "real" Perl code can your parser handle?
- Where are the hard boundaries of static analysis?
- Could runtime context (eval, source filters) ever be integrated?

---

### Q17: What's the weirdest Perl syntax you've had to handle?
**The Question**: Parser error buckets and corpus findings must have surfaced some truly bizarre Perl patterns. What's the most exotic syntactic construct the parser handles?

**Why it's interesting**: This reveals the real-world messy complexity the parser navigates. It also likely reveals what initially broke the tree-sitter approach.

**Evidence from codebase**:
- docs/issues/corpus/gaps/ lists 6+ categories of unhandled patterns
- Memory references "phantom bucket #5" — undiscovered error category
- Parser source has specific functions for: substitution operators (ADR-0001), heredocs (ADR not found), regex, quotes, complex paren arguments
- Git history shows fixes like "handle complex expressions in parenthesized arguments (#1704)"
- Known issues reference: "contextual slash/division regex disambiguation," "catastrophic regex backtracking," "deep nesting stack overflow"

**Follow-up**:
- Which Perl feature causes the most parser fragility?
- Have you had to special-case things that feel like "this shouldn't need special code"?

---

### Q18: 72% CPAN coverage — what's in the remaining 28%? Source filters?
**The Question**: The CPAN corpus baseline shows 72-80% of real-world Perl parses cleanly. What's the distribution of failures in the remaining 20-28%? And is source filter code execution a significant portion?

**Why it's interesting**: The gap between "good" (80%) and "complete" (100%) reveals what Perl use cases are out of scope. This sets realistic expectations for users.

**Evidence from codebase**:
- FIVE_ERAS.md: "CPAN corpus parse rate: 72% to 80% (3,139 to 3,484 clean files out of 4,355)"
- docs/issues/corpus/gaps/ lists: timeout-hang-risks, nodekind-never-seen, ga-feature-missing-coverage
- Specific gap file: "timeout-hang-risks/source-filter-code-execution.md" references "Perl's source filter mechanism allows code execution"
- KNOWN_ISSUES.md mentions: `use Perl6::Say;  # Adds 'say' keyword via source filter`

**Follow-up**:
- Is the 28% mostly "we can't parse this statically" or "we haven't implemented this yet"?
- What's your strategy for source filters? (Give up? Heuristics? Light eval?)
- If CPAN coverage hit 99%, would you consider the parser "done"?

---

## DEVELOPMENT MODEL & METHODOLOGY

### Q19: The scout-constrain-build pattern — is it specific to this project, or a general principle?
**The Question**: You discovered that scoping tasks precisely (scout) before building leads to 90% success vs 50%. Is this a universal principle for AI-assisted development, or specific to parser work?

**Why it's interesting**: This is a claim about how AI work should be structured. Understanding its generality reveals whether it's a deep insight or a heuristic that happened to work here.

**Evidence from codebase**:
- FIVE_ERAS.md documents the pattern: "Scout: Understand the problem. Constrain: Convert understanding into specification. Build: Execute within constraints."
- Memory notes: "feedback_explore_cheap_build_expensive.md — Exploration and planning are cheap; invest heavily before building"
- Also: "feedback_research_before_build_pattern.md — Scout root causes FIRST, then use findings verbatim as builder prompts"
- Mentioned in swarm.md as the canonical pattern

**Follow-up**:
- Have you tried it on non-parser domains? (LSP features, DAP, documentation)
- What's the scout:builder ratio you've converged on? (1:3? 3:1? Task-dependent?)
- Would you teach this to another team?

---

### Q20: "Draft first, review before CI triggers" — why is that order important?
**The Question**: The feedback documents mention draft PRs before review, review before CI. This is counter to "merge to master and fix CI." Why is the order non-negotiable?

**Why it's interesting**: This is a process choice that affects how human attention is deployed. It reveals assumptions about where errors hide.

**Evidence from codebase**:
- Memory: "feedback_draft_prs_first.md — Draft PRs first; review before CI triggers"
- swarm.md: "Draft first: Builders create draft PRs. Reviewer marks ready after checking."
- Also: "feedback_review_catches_real_bugs.md — Review caught 15+ real bugs; never skip review"
- CLAUDE.md mentions draft PR creation flow

**Follow-up**:
- What real bugs did review catch that CI would have missed?
- What's the cost of draft PRs in terms of overhead?
- Have you considered auto-merging to main with safety checks instead?

---

## THE META QUESTIONS

### Q21: You're a CPA, not a traditional developer. How does that background shape how you think about this project?
**The Question**: Accounting trains you to think about accuracy, audit trails, reconciliation, and measurement. How does that background inform your approach to code quality, metrics, and process?

**Why it's interesting**: This is asking about identity and perspective. The metrics and accounting metaphors throughout the codebase (DevLT, receipts, ledgers, debt) suggest this isn't coincidental.

**Evidence from codebase**:
- AGENTIC_DEV.md uses explicit budget language: "Budget Model," "DevLT bands," "Compute Cost bands"
- Reference to "tech debt ledger" (.ci/debt-ledger.yaml)
- Concept of "receipts" — test output, CI results proving claims (audit trail metaphor)
- LESSONS.md as a wrongness log (audit/reconciliation mindset)
- Memory note: "feedback_metrics_pipeline_broken.md — swarm-metrics.jsonl barely used; need hook-based auto-logging"

**Follow-up if answer is personal**:
- What accounting principles do you think developers often miss?
- Would you recommend other developers learn accounting?
- Does being a CPA make you more or less comfortable with ambiguity?

---

### Q22: Did you ever want to give up on this project?
**The Question**: This is an honest question about the journey. Building a language server for Perl, in Rust, with 100 agents in parallel — there must have been moments of doubt. When did you come closest to abandoning it?

**Why it's interesting**: This is a vulnerability question. The answer will be honest about motivation, resilience, and what kept you going.

**Evidence from codebase**:
- No direct evidence, but the five-era progression shows intentional deceleration and reflection (Era 3)
- The metrics pipeline is "barely used" — suggesting friction or unmet expectations at some point
- The swarm improvements document suggests cycles of learning and iteration

**No quote to cite, but this is a genuine question about the human behind the code.**

---

### Q23: What's the most important thing the project has taught you that you didn't expect to learn?
**The Question**: You probably started with assumptions about how to build a Perl LSP, how to use AI, what matters for developer experience. What did you get most wrong?

**Why it's interesting**: This reveals gaps between intent and outcome. It's likely the most honest and useful insight.

**Evidence from codebase**:
- The oscillation between velocity and quality (Eras 1-3 vs 4-5) suggests you learned something about the speed-quality tradeoff
- The scout-constrain-build discovery came from experience, not theory
- The dual-indexing decision shows Perl-specific learning
- The memory system's existence suggests you learned the value of institutional knowledge across sessions

**No direct quote, but the learning artifacts are everywhere.**

---

## TECHNICAL DEEP DIVES

### Q24: The substitution operator parsing is notoriously complex. What's your solution to `/s/foo/bar/`?
**The Question**: Perl's substitution operator (`s///`) is grammatically ambiguous — the delimiters can be almost anything, and parsing depends on context. ADR-0001 documents this. What approach did you settle on?

**Why it's interesting**: This is a concrete example of Perl's complexity. The answer reveals how you balance simplicity with completeness.

**Evidence from codebase**:
- ADR-0001: "Substitution operator parsing architecture"
- File path indicates complexity: `crates/perl-parser-core/src/engine/parser/expressions/substitution.rs` or similar
- The fact that an ADR exists suggests this was a significant design decision

**Follow-up if technical**:
- What percentage of CPAN files use non-standard delimiters?
- Did you handle s{foo}{bar}? s|foo|bar|? s!foo!bar!?
- What about nested delimiters in the replacement string?

---

### Q25: The lexer is "context-aware." What does that mean, and why is it necessary?
**The Question**: Traditional lexers are context-free. Perl's lexer needs to understand context (are we in a quote? a regex? a block?). What context does your lexer track?

**Why it's interesting**: This is the bridge between parsing difficulty and implementation. Understanding the lexer's context awareness reveals the core of the parsing strategy.

**Evidence from codebase**:
- ADR-0014: "Mode-aware lexer"
- Lexer crate: `crates/perl-lexer/`
- Git commits reference "context-aware tokenizer"
- The need for context-aware tokenization is implied by the quote, regex, heredoc, and substitution-specific crates

**Follow-up**:
- How do you handle quote nesting? (q{a{b{c}}}?)
- What's the lexer's state machine? (How many states?)
- Does the lexer ever need to backtrack?

---

### Q26: Incremental parsing — is it in the LSP, or deferred as future work?
**The Question**: True incremental parsing (re-parsing only changed regions) is a holy grail for language servers. Does perl-lsp do this, or does it re-parse the full file on each change?

**Why it's interesting**: Incremental parsing is what makes LSPs responsive. If perl-lsp doesn't do it, that's a known limitation. If it does, that's a remarkable achievement.

**Evidence from codebase**:
- ADR-0010: "Incremental Parsing Architecture" — exists, suggesting this was designed
- Rope data structure mentioned: ADR-0020: "Rope Document Management"
- Memory references "incremental parsing" in test coverage topics

**Follow-up if yes**:
- What's your invalidation strategy? (Line-based? AST-based?)
- How do you handle multi-line edits?
- What's the performance win vs full re-parse?

---

## FUTURE DIRECTION

### Q27: What does "best Perl LSP" look like to you?
**The Question**: You have a vision of what's possible. What's the destination? What would it take for you to declare this project "done"?

**Why it's interesting**: This reveals whether you're building to a specific north star or iterating opportunistically.

**Evidence from codebase**:
- ROADMAP.md exists (referenced but not quoted)
- features.toml catalogs capabilities
- CURRENT_STATUS.md tracks completeness
- The 72% → 80% CPAN progression suggests a ratcheting approach
- "Public alpha" designation suggests this is not yet the final form

**Follow-up**:
- Is "best Perl LSP" measured by CPAN coverage, user satisfaction, or feature parity with other LSPs?
- What would get you to 1.0?
- Is perfection (100% CPAN coverage) a goal or known unrealistic?

---

### Q28: Is the swarm methodology the product, or is perl-lsp the product?
**The Question**: You've spent significant effort on the swarm (skills, coordinators, memory system, workflow). Is that infrastructure upstream innovation that other projects could reuse, or is it in service of perl-lsp specifically?

**Why it's interesting**: This reveals your thinking about leverage and impact. Is the goal "best Perl LSP" or "how AI agents collaborate effectively"?

**Evidence from codebase**:
- Memory: "feedback_swarm_is_the_product.md — Swarm and codebase are equal goals; improving swarm compounds across all sessions"
- Skills are documented in `.claude/skills/`, agents in `.claude/agents/`
- CLAUDE.md is explicit about the orchestration model
- The fact that this methodology is written down and versioned suggests it's treated as a product

**Follow-up**:
- Have you thought about open-sourcing the swarm methodology?
- Could the scout-constrain-build pattern work for other projects?
- Do you think agent teams are the future of software development?

---

### Q29: Could this approach work for other language servers?
**The Question**: If someone wanted to build a Python LSP or a Go LSP using this methodology, what would transfer, and what would need to be rebuilt?

**Why it's interesting**: This asks whether perl-lsp is a one-off or a template. The answer reveals what's language-specific vs. universal.

**Evidence from codebase**:
- The microcrate architecture is language-agnostic
- The scout-constrain-build pattern is language-agnostic
- But Perl-specific crates (regex, heredoc, quote, substitution) wouldn't transfer
- The CPAN corpus work is Perl-specific

**Follow-up**:
- What would the first 20% of language-agnostic work be?
- Would you publish a template?

---

## LAUNCH STRATEGY

### Q30: Who is your ideal perl-lsp user?
**The Question**: Perl developers span decades of experience and use cases (system admin, bioinformatics, web, legacy maintenance). Who are you building this for first?

**Why it's interesting**: Launch strategy reveals priorities. First users shape feedback and feature direction.

**Evidence from codebase**:
- README addresses both users and builders: "If you want editor features, install perl-lsp. If you want to build Perl-aware Rust tooling, start with perl-parser."
- Suggests dual audience (Perl devs and Rust developers building Perl tools)
- Public alpha positioning suggests broad availability but not final

**Follow-up**:
- What's your distribution strategy? (crates.io? VSCode extension? Linux package managers?)
- Are there enterprises or teams you want to reach first?
- What feedback would make you pivot?

---

### Q31: Why open source? Why not build this as a commercial product?
**The Question**: A Perl LSP is valuable to enterprises. You could have monetized this. What made you open source it?

**Why it's interesting**: This reveals values and philosophy. Open source enables community. Commercial could enable focus.

**Evidence from codebase**:
- Repository is public on GitHub under EffortlessMetrics org
- MIT/Apache-2.0 dual license
- Installation instructions are public
- CONTRIBUTING.md exists

**No quote about why, but the choice is clear.**

**Follow-up if answer is values-driven**:
- How do you sustain the project long-term?
- Is there a business model around perl-lsp, or is it purely volunteer/educational?

---

### Q32: The VSCode extension is in this repo. Why keep it together with the server?
**The Question**: Many LSPs separate the server (polyglot) from the editor-specific extension. You kept them together. Why?

**Why it's interesting**: This is a packaging and maintenance choice that affects what new contributors see and how the project scales.

**Evidence from codebase**:
- `vscode-extension/` exists in the repo
- README lists it as part of the quick start
- The extension README is listed as relevant documentation

**Follow-up**:
- Do you have clients for Neovim, Emacs, etc.?
- Is the extension bundled with the server binary, or separate installation?
- Would you accept a third-party extension?

---

## FINAL QUESTIONS (OPEN-ENDED)

### Q33: What question do you wish someone would ask you about this project?
**The Question**: There's probably a story or insight you have that hasn't come up yet. What is it?

**Why it's interesting**: This invites the person being interviewed to share what they think is most important.

**Evidence from codebase**: None — this is an open-ended slot.

---

### Q34: If you could restart this project knowing what you know now, what would you do differently?
**The Question**: Hindsight reveals inefficiencies and wrong turns. What's the single biggest thing you'd change?

**Why it's interesting**: This is honest reflection. It likely reveals the highest-leverage insight.

**Evidence from codebase**:
- The five-era progression shows evolution of approach
- Memory notes on learnings (feedback_* files) document things tried and adjusted

---

### Q35: What's the one thing about Perl that surprised you most?
**The Question**: Most developers approach Perl with preconceptions (old, messy, legacy). What actually surprised you?

**Why it's interesting**: This reveals fresh perspective on Perl. It's often the most interesting part of interviews with language implementers.

**Evidence from codebase**:
- PARSING_PERL.md catalogues complexity
- The fact that dual indexing (bare + qualified) is necessary suggests Perl's naming flexibility is significant
- Source filters, prototypes, and context-sensitivity all create complexity

---

---

## INTERVIEW STRUCTURE & PACING

**Recommended Interview Approach:**

1. **Origin Story (Q1-3)**: 10 minutes — establish motivation and initial architectural choice
2. **Five Eras (Q4-8)**: 15 minutes — understand the velocity paradox and what made Era 5 different
3. **Architecture Decisions (Q9-12)**: 10 minutes — dive into technical choices that enable the swarm
4. **Swarm Experience (Q13-15)**: 10 minutes — human perspective on coordinating 100 agents
5. **Perl Specifics (Q16-18)**: 10 minutes — what makes Perl hard and how you solved it
6. **Methodology (Q19-23)**: 10 minutes — the meta-insights about AI-assisted development
7. **Future (Q27-32)**: 10 minutes — vision and launch strategy
8. **Reflection (Q33-35)**: 5 minutes — what wasn't asked that should have been

**Total: ~80 minutes for a comprehensive interview**

**For Shorter Formats:**
- **30-minute interview**: Q1-3, Q5-6, Q16-17, Q27-28 (origin, pivot, perl challenges, vision)
- **15-minute interview**: Q1, Q5, Q8, Q16, Q27 (motivation, copilot→claude, success pattern, perl hard part, future)

---

## ARTICLE HOOKS FROM THESE QUESTIONS

- **"The Velocity Paradox: How Slowing Down Made Perl LSP 100x Faster"** (Q4, Q5, Q7)
- **"Scout-Constrain-Build: Why 90% of AI Code Works When Scoped Precisely"** (Q8, Q14, Q19)
- **"100 Agents: What I Learned Coordinating a Swarm"** (Q13, Q14, Q15)
- **"Why a CPA Built a Perl Language Server"** (Q21, and weave Q7 throughout)
- **"Only Perl Can Parse Perl — But Not For Much Longer"** (Q16, Q17, Q18)
- **"The Five Eras of AI Development"** (entire Q4-Q8 arc, with references to FIVE_ERAS.md)

---

*These questions are designed to elicit stories, insights, and learning that go beyond the codebase. They assume Steven Zimmerman will have thoughtful, experience-based answers that reveal not just what was built, but why and how it changed what's possible in AI-assisted development.*

# Interview Questions: Building perl-lsp with AI Agents

*Questions and answers for discussing the perl-lsp project, its development methodology, and what was learned about AI-assisted software development at scale.*

---

## Project Overview

### Q: What is perl-lsp?

perl-lsp is a Language Server Protocol (LSP) implementation for Perl, written in Rust. It provides IDE features — completion, go-to-definition, hover, diagnostics, rename, code actions, and more — for Perl code. It includes a hand-written recursive descent parser, a Debug Adapter Protocol (DAP) server, and a VSCode extension.

The project has 563K lines of Rust code across 133 workspace crates, 2,768 commits, and implements 53 LSP features. It parses 80%+ of the CPAN top-1000 modules cleanly, targeting 90%+ for the 0.12.0 public alpha.

### Q: Why Perl? Isn't it a dying language?

Perl has ~200K-500K active developers and an enormous installed codebase. Every existing Perl language server requires a Perl installation and offers limited features. 78% of Perl developers use no LSP integration at all — not because they don't want it, but because the available tools don't meet modern expectations.

perl-lsp fills a genuine gap: zero-dependency, cross-platform, feature-rich IDE support for a language that deserves better tooling.

### Q: Why Rust?

Three reasons:
1. **Performance**: A Perl parser must handle large files and complex constructs without lag. Rust's zero-cost abstractions and lack of garbage collection provide predictable, fast parsing.
2. **Safety**: The parser processes untrusted input (arbitrary Perl code). Rust's type system and borrow checker prevent entire categories of bugs (buffer overflows, use-after-free, data races).
3. **Distribution**: A single native binary with no runtime dependencies. `cargo install perl-lsp` and you're done. No Perl, no Python, no Node.js.

---

## The Parser

### Q: "Only perl can parse Perl" — how do you handle that?

Larry Wall was right: Perl's grammar is context-sensitive, and some constructs require executing code to parse correctly. A static parser will never be 100% correct.

We handle it by being correct enough for IDE features. The parser uses a mode-based lexer (`LexerMode` in `crates/perl-lexer/src/mode.rs`) that tracks whether to expect a term or an operator. This solves the `/ ` ambiguity (division vs. regex), the `{}` ambiguity (hash vs. block), and most other context-sensitive constructs.

We test against 4,355 real CPAN modules. Currently 80%+ parse cleanly. The remaining failures are categorized into error buckets with specific root causes and fix plans. The 2-3% floor consists of source-filtered code and `BEGIN` block side effects — genuinely unfixable by static analysis.

### Q: You tried three parsers. What did v1 and v2 teach you?

**v1 (tree-sitter)** taught us that Perl's grammar is too context-sensitive for GLR parsers. The external scanner (`scanner.c`) grew to 975 lines of hand-written C — 3-5x larger than tree-sitter scanners for JavaScript or Python — and was still incorrect for many cases. The fundamental issue: tree-sitter's lexer cannot query the parser's state.

**v2 (Pest/PEG)** taught us that PEGs handle some ambiguities with ordered choice and unlimited lookahead, but cannot maintain state between alternatives. When parsing `{ ... }`, a PEG can try "hash" then "block," but it cannot carry context (like "we just saw `sort`") into the choice. Performance was also a problem — PEG backtracking on deeply nested constructs caused exponential behavior.

**v3 (recursive descent)** works because a hand-written parser can do everything tree-sitter and PEG cannot: maintain a state machine in the lexer, pass context through function arguments, peek ahead multiple tokens, and call specialized parse functions based on the current construct. The trade-off is maintenance cost — every rule is hand-coded.

### Q: What are the hardest Perl constructs to parse?

1. **`/` (division vs. regex)**: Same character, two meanings. Requires tracking whether the previous token was a term or an operator.
2. **`{}` (hash vs. block vs. bare block)**: Braces have three meanings. Disambiguation requires knowing whether we're in expression or statement context, and whether the preceding token was `sort`/`map`/`grep`.
3. **Heredocs**: The body starts on the next line, but the current line continues. Multiple heredocs on one line interleave their bodies. Requires a deferred-token queue.
4. **`print STDERR "msg"`**: Is `STDERR` a function argument or a filehandle? Requires knowing that `print` has special syntax.
5. **Source filters**: Modules that rewrite source code before parsing. Unfixable by static analysis.

---

## The Swarm Methodology

### Q: How does the swarm work?

One human sets direction. An orchestrator (Claude Opus) translates direction into agent tasks. Up to 100 agents work in parallel on isolated worktrees, each modifying a different crate.

The pipeline has seven stages:
```
Signal → Plan → Build → Review → Gate → Deploy → Wisdom
```

Each stage has a defined agent role, artifact, and handoff protocol. Scouts create issues. Builders create PRs. Reviewers check quality. CI gates enforce contracts. Ops agents merge in batches. The wisdom stage captures learnings as memory.

### Q: How do 100 agents work without conflicting?

The microcrate architecture. The workspace has 133 crates, each in its own directory. Each agent works on one crate in an isolated git worktree. Git worktree isolation + crate directory isolation = zero file conflicts.

This was discovered empirically: early swarm sessions had constant merge conflicts when agents modified shared files. Extracting each module into its own crate eliminated the conflicts. 32 crates were extracted in a single day (March 5, 2026). The architecture followed the workflow.

### Q: What's the optimal number of agents?

About 9 coding agents, plus scouts and reviewers.

The math: the merge queue is 3-wide (rapid merges cancel each other's CI runs). CI cycles take ~5 minutes. Maximum merge throughput: 36 PRs/hour. Average agent work time: ~15 minutes. Optimal coding agents: 36 / (60/15) = 9.

Beyond 9 coding agents, you produce PRs faster than CI can merge them. The excess capacity should go to scouts (research, no PRs), reviewers (quality assurance, no PRs), and planners (strategy, no PRs).

In practice, the platform has a ~75 named teammate ceiling. The roster allocation should be roughly:
- 9 coding agents (12%)
- 15 scouts (20%)
- 5 reviewers (7%)
- 10 reserve (13%)
- Remaining: ops, improvers, planners

### Q: What's the human's role?

Strategic direction. The human decides:
- What to prioritize (corpus improvement? new features? distribution?)
- What quality bar to enforce
- When to stop
- What to learn from failures

The orchestrator translates these decisions into agent tasks but never makes strategic decisions. When the human disengages, the swarm drifts — in cycle 1, agents polished P3/P4 features while the P1 corpus goal didn't move. In cycle 5, active direction drove corpus from 72% to 80%+ and filed 80+ issues.

The swarm amplifies human judgment. It does not replace it.

### Q: What's the failure rate?

It depends on task constraint:
- **Parser fix agents** (TDD, one crate, clear test cases): ~90% success
- **Draft PR fixers** (rebase + verify existing code): ~100% success
- **Feature agents** (new abstractions, cross-crate work): ~50% — many produce compile errors

The key insight: research converts unconstrained work into constrained work. A 10-minute scout that identifies exact function names, file paths, and API signatures transforms "implement feature X" into "add method Y to file Z." The scout's output IS the constraint.

---

## Scaling and Limits

### Q: What broke at scale?

1. **Merge queue saturation**: 100 agents producing PRs overwhelms a 3-wide merge queue. Batches of 3, wait for CI, repeat — 85 minutes to process 50 PRs.
2. **Stale baselines**: Corpus metrics showed 72% when the actual clean rate was 85%+ because nobody ran the ratchet command.
3. **Phantom metrics**: Error bucket #5 was misclassified — 83 files attributed to a bug category that didn't exist in the parser.
4. **policy_checks friction**: Every test-adding PR failed CI because computed documentation (test counts) went stale.
5. **Diminishing returns past 50 agents**: Agents 51-100 produced valid but low-value test coverage work.

### Q: What would you do differently?

1. **Extract LSP providers into crates earlier**: Phase 5 work should have been Phase 2. Provider extraction is the highest-impact change for swarm parallelism.
2. **Automate ratcheting**: Manual ratcheting creates a gap between reality and metrics. Automate it.
3. **Fix CI first, always**: In cycle 4, clippy failures on master blocked all PRs. Should have been the first fix, not an afterthought.
4. **Agent count budget upfront**: Plan how many agents of each type, don't spawn until the ceiling hits.
5. **Validate metrics before acting**: The phantom bucket consumed scout time investigating a classification artifact.

---

## Architecture Insights

### Q: 133 crates for 563K lines — isn't that over-engineered?

For traditional development, yes. Industry guidance would suggest 30-50 crates.

For swarm development, it is essential. Each crate is an independent unit of work. 100 agents can work on 100 crates with zero conflicts. The build system, test system, and dependency graph all operate at crate granularity. A change to `perl-lsp-hover` only recompiles that crate and `perl-lsp` — not the entire workspace.

The "over-engineering" criticism assumes a single developer or small team. Under swarm development, each crate boundary is a concurrency boundary. The architecture IS the parallelism enabler.

### Q: What's the biggest technical debt?

`perl-tdd-support` — the test utilities crate with 62 reverse dependencies. It accumulates test helpers for every domain (parser, LSP, DAP). Any change triggers recompilation of half the workspace. Should be split into domain-specific test utility crates: `perl-tdd-parser`, `perl-tdd-lsp`, `perl-tdd-assertions`.

### Q: What's the biggest architectural insight?

The mode-based lexer. Tree-sitter failed because its lexer cannot query the parser's state. Perl requires the lexer to know what the parser is doing (`LexerMode.ExpectTerm` vs `LexerMode.ExpectOperator`). The recursive descent parser passes this state explicitly, making lexer-parser coupling a feature rather than a bug.

This insight applies beyond Perl: any language with context-sensitive lexing (C, C++, templates in many languages) benefits from a mode-based lexer over a context-free tokenizer.

---

## Meta-Questions

### Q: Is this replicable?

The methodology is replicable. The specific architecture (133 microcrates) emerged from Perl's parsing complexity and the swarm workflow. A different project would likely have different crate boundaries but the same principles:
- Isolation enables parallelism
- Scouts before builders
- Metrics must be validated
- The merge queue is the bottleneck
- Constraints drive innovation

### Q: What's the cost model?

Traditional: $150-250/hr senior developer, serial execution, 3-8 changes/day.
Swarm: $1-5 per agent flow, parallel execution, 40-80 changes/session.

The metric that matters is DevLT (Developer Lead Time): minutes of human attention per trusted change that reaches production. In swarm development, DevLT drops from hours to minutes.

### Q: What's the most surprising thing you learned?

Two agents independently fixing the same bug (PR #1903 and #1906 both fixing prototype mode parsing) is not waste — it is parallel solution space exploration. The simpler patch in #1903 worked. The state machine approach in #1906 was architecturally superior. Without running both, we would have shipped the inferior solution.

Duplication has value. The cost of one extra agent (~15 minutes of compute) is small. The benefit of discovering the better architecture is large.

---

## Questions About the Future

### Q: What comes after 0.12.0?

Path to 100% CPAN corpus coverage (see `CORPUS_ROADMAP.md`):
- Phase A: 90% (5 builders, 4-5 weeks)
- Phase B: 95% (12-15 builders, 7-9 weeks)
- Phase C: 98% (20+ builders, 11-15 weeks)
- Unfixable floor: 2-3% (source filters, BEGIN blocks)

Plus: VSCode extension publication, Homebrew formula, perlcritic/perltidy integration, Neovim/Emacs/Helix integration guides.

### Q: Will the swarm methodology evolve?

Yes. Every cycle discovers improvements. The methodology is itself a product being developed alongside the codebase. ~20% of agent capacity is always reserved for self-improvement (better skills, better hooks, better prompt templates). The swarm is the product as much as the LSP is.

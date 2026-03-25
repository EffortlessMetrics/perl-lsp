# Code Is Cheap; Trusted Change Is Not

## A methodology for making AI-generated code trustworthy by construction, not by hope

---

### The Attention Bottleneck

Software development does not parallelize well at the review stage. Ten engineers can write ten patches simultaneously. They cannot review each other's work simultaneously — not without coordination overhead that eats the time saved.

This bottleneck exists in human teams. It still exists with AI agents. LLMs have made code generation nearly free, but they have not changed the economics of trust. A patch that has not been reviewed, tested, formatted, and approved is not a change — it is a candidate. The gap between candidate and change is where almost all the time goes.

The perl-lsp project was an attempt to compress that gap without sacrificing the trust it exists to provide.

---

### What Was Built

perl-lsp is a Perl language server written in Rust — a native binary that provides IDE features (completions, navigation, diagnostics, debugging) for Perl 5 without requiring a Perl installation.

The codebase:

- **~598,000 lines of Rust** across **134 workspace crates** (as of 2026-03-25; run `just verify-publication-facts` for the current value)
- **98 LSP and DAP features**, all at GA maturity
- **87% mutation score** (industry baseline: 60-70%)
- **90.9% CPAN corpus manifest coverage** (85.7% baseline clean-parse rate) across 4,355 real-world Perl modules
- **Zero panics** in production code, enforced by Clippy policy
- **Less than 50ms** LSP response time under load

The project ran for **9 months** (July 2025 through March 2026), driven by one human (Steven Zimmerman) and a swarm of AI agents — peaking at 75-100 agents in a single working session.

---

### The Cost Model Nobody Measures

The standard way to measure developer productivity is output per unit time: commits per day, PRs per week, lines of code per sprint. These metrics capture throughput. They do not capture trust cost.

**DevLT** (Developer Lead Time) is a different measurement: minutes of human attention per trusted change (merged PR).

For a traditional senior developer team:
- 1 senior reviewing 1 PR: 30-60 minutes (read the diff, run tests, check design, ask questions, approve)
- 3-person team reviewing collaboratively: 60-120 minutes accounting for async delays
- Per-week output: 8-15 PRs with a well-functioning team

For the perl-lsp swarm, at peak velocity:
- **56 PRs created in 5 days** (11.2 PRs/day)
- **190+ PRs merged over the 9-month project**
- **Human time: approximately 150-200 hours** across the entire project
- **DevLT: 3-5 minutes per merged PR**

The 3-5 minute figure is not the time to *create* a PR. It is the human review and merge decision time. Agent-based review, automated testing, and CI gates handle the rest — and they do so in parallel, not serially.

---

### The Economics

| Dimension | Swarm (perl-lsp) | Traditional Senior Rust Team |
|-----------|-----------------|------------------------------|
| Calendar time | 9 months | 35-45 months (estimated) |
| Human attention | 150-200 hours | 3,500-4,600 hours |
| Compute cost | ~$20K (estimated) | $0 (payroll-funded) |
| Total cost | $40K-79K | $500K-$1.2M |
| Velocity (peak) | 11.2 PRs/day | 0.4-0.8 PRs/day |
| DevLT | 3-5 min/PR | 60-120 min/PR |
| Efficiency gain | baseline | — |

The comparison project is rust-analyzer — a similar scope Rust language server built by ~800 contributors over 5 years. perl-lsp is smaller but comparable in architecture: parser, LSP protocol compliance, workspace indexing, semantic analysis, debugging.

The traditional cost estimate ($500K-$1.2M) is built from first principles: 35-45 developer-months at Rust developer rates ($90-125/hour productive time, industry standard for San Francisco market). This is a wide range deliberately — the honest answer is that it depends heavily on the team and the requirements. The narrow claim is that it would not be $79K.

The swarm cost ($40K-79K) breaks down as approximately $20K compute (estimated from token consumption across ~500 agent sessions) plus $24K-50K of human time at $150-250/hour. The human time figure is the most uncertain — memory files from the project record 5 major development cycles and ~30 planning sessions, but exact hours are not tracked.

**Confidence levels**: The codebase metrics (~598K LOC, 134 crates, 200+ PRs) are facts from git. The velocity figure (56 PRs in 5 days) is documented in memory files from the session. The cost estimates are informed approximations (Tier C), not accounting-precision figures — see `docs/articles/research/COST_ROI_ANALYSIS.md` Section 9 for confidence intervals and methodology.

---

### What Made the Economics Work

Four architectural decisions made the swarm economics possible. None of them were obvious at the start of the project.

**1. Microcrate architecture (134 crates, zero circular dependencies)**

A monolithic codebase cannot support 75 parallel agents. If every agent touches shared files, merge conflicts are guaranteed. 134 crates — each with a single responsibility and a narrow public API — means that most pairs of agents are working in separate crates entirely. No conflict is possible.

This was not planned from day one. It emerged from Era 3 (October 2025 through February 2026), a deliberate slowdown to extract god files into microcrates before scaling up. The slowdown cost 4 months. Without it, Era 5's 75-agent sessions would have been unusable.

**2. Scout-Constrain-Build pattern (90% success rate on constrained tasks vs 50% unconstrained)**

An agent given a vague task succeeds roughly half the time. An agent given a precise spec — specific file, specific line, specific root cause, specific test to write — succeeds roughly 9 times in 10. The difference is a scout agent that spends 10 minutes characterizing the problem before a builder agent spends 60 minutes fixing it.

The ratio is: one scout per three builders for novel work, one scout per ten builders for well-understood patterns. The scouts are cheap (smaller models, short sessions); the builders are where the compute cost concentrates.

**3. Automated review agents (98% pre-merge catch rate)**

Code review is where serial bottlenecks traditionally form. Review agents — agents whose explicit task is to read a PR, find problems, and push fixes directly to the branch — parallelize the review function. A human reviewer's job becomes: confirm the review agent did its work, check the CI gate, and approve the merge.

The human is not removed from the loop. The human's role in the loop is compressed from "full reviewer" to "final gatekeeper." That compression is where the DevLT number comes from.

**4. Worktree isolation (safe parallel iteration)**

Each agent works in its own git worktree — a separate checkout of the repository. A failed agent's work cannot damage the main branch. A reviewer agent can push fixes directly to a PR branch without affecting other agents working simultaneously. Failed attempts are abandoned, not reverted. The retry cost is a new agent spawn, not a git surgery operation.

---

### The Quality Gate

A system that generates trusted change must be honest about what "trusted" means. The perl-lsp project used three mechanisms:

**Mutation testing (87% mutation score).** Mutation testing modifies the production code (flips a comparison, changes a return value) and checks whether the test suite catches the mutation. A test suite that passes on mutated code is not testing what it claims to test. 87% mutation score means that 87% of the time, the tests catch a deliberate code fault. Industry baseline is 60-70%.

**CPAN corpus ratchet.** Every PR is validated against 4,355 real CPAN Perl modules. The clean-parse rate can only increase — any PR that degrades corpus coverage is blocked. This prevents the common failure mode of local test suites that pass while real-world correctness degrades.

**Zero-panic policy.** `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented!()` are banned in production code by Clippy policy (deny-level, compile errors across the workspace). An LSP server crash is silent — the editor spawns the server in the background, and when it panics, the user sees degraded completions and navigation with no error dialog. Zero panics means zero silent crashes.

---

### The Attention Compression

The core result is this: the 9-month perl-lsp project required approximately 150-200 hours of human attention to produce a codebase that would have required 3,500-4,600 hours in a traditional senior developer model.

That is a 17-23x compression of human attention.

The compute cost is real — $20K in estimated API costs — and the traditional team's cost is also real: $500K+ in developer salaries. Trading $20K in compute for $480K in avoided salary cost is a 24x return on the compute investment.

But the more interesting number is not the money. It is the time. A project that would have taken 35-45 months of sustained development completed in 9 months. The speedup is 4-5x on calendar time, enabled entirely by parallel agents working on isolated microcrates.

The key quote from this project:

> "Code is cheap; trusted change is not. Anyone can generate a patch. LLMs have made that nearly free. The expensive part is everything that turns a patch into a change you'd bet production on."

The perl-lsp methodology — scout-constrain-build, automated review, worktree isolation, CPAN ratchet, mutation testing — is an answer to that expensive part. It is not about generating more code. It is about making generated code trustworthy faster.

---

### What This Does Not Prove

This is one project, built by one person with a specific set of skills, on a specific type of problem (language tooling) with characteristics that made the approach work well (microcrate-amenable, well-specified LSP protocol to implement).

The economics do not automatically transfer to:

- **Correctness-critical systems** (cryptography, financial settlement, safety-critical code) where the cost of a missed defect exceeds the cost of slow, careful human review
- **Cross-team coordination problems** where the bottleneck is human agreement, not code generation
- **Exploratory work** with undefined problem spaces, where scouts cannot characterize tasks precisely enough for constrained builders
- **Large organizations** where the overhead of managing 75 agents exceeds the overhead of managing 5 experienced engineers

The scout-constrain-build pattern requires good scouts. Good scouts require domain knowledge in the orchestrator — the human directing the swarm must understand the problem well enough to know whether a scout's spec is credible. The 90% success rate on constrained tasks assumes the constraints are correct.

---

### The Headline Numbers

- **9 months** to build what would have taken 35-45 months traditionally
- **$40K-79K** total cost vs $500K-$1.2M traditional equivalent
- **3-5 minutes** of human attention per merged PR (DevLT)
- **87%** mutation score on a production codebase
- **134 crates**, zero merge conflicts across 100 parallel agents
- **56 PRs** created in 5 days at peak velocity

The project is public at [github.com/EffortlessMetrics/perl-lsp](https://github.com/EffortlessMetrics/perl-lsp). The methodology is captured in `docs/articles/SWARM_METHODOLOGY.md`. The economics are documented with confidence intervals in `docs/articles/research/COST_ROI_ANALYSIS.md`.

---

*Metrics verified against `docs/project/PUBLICATION_FACTS_LEDGER.md`. Cost estimates are informed approximations — see `docs/articles/research/COST_ROI_ANALYSIS.md` Section 9 for confidence intervals and methodology.*

# The Three-Layer Product

*perl-lsp is not one product. It is three products in one repository --- and that is what makes it unusual.*

---

## The Observation

If you browse the perl-lsp repository expecting a language server, you will find one. You will also find 48 slash commands, 15 skills, 106 memory files, 7 ADRs, a corpus pipeline, a swarm methodology document, and a `.claude/` directory that is itself a small operating system.

This is confusing until you see the structure. The repository contains three distinct layers of work, each a product in its own right, each with its own users, its own quality standards, and its own feedback loops. They happen to live in the same git history because they co-evolved --- and because separating them would destroy the property that makes all three valuable.

> *"The repo is no longer just improving code. It is improving the machinery that improves code, and then improving the memory that explains both."*

---

## Layer 1: The LSP

This is the product that users install. A Perl Language Server Protocol server, a Debug Adapter Protocol server, a recursive descent parser, and a VSCode extension. The thing that makes Perl development better in an editor.

The numbers:

| Metric | Value |
|--------|-------|
| Lines of Rust | 591,034 |
| Workspace crates | 133 |
| Parser generations | 3 (C/tree-sitter, Pest PEG, recursive descent) |
| LSP features governed | 98 |
| LSP 3.18 compliance | 99% |
| CPAN corpus manifest coverage | 90.9% |
| Commits | 3,200+ |
| Pull requests | 2,646+ |

Layer 1 is a serious piece of software. Three parser generations taught a lesson --- only recursive descent was flexible enough for Perl's context-sensitive syntax. The microcrate architecture (133 crates, zero circular dependencies, average crate size ~4,450 lines) was an intentional investment in modularity that took months of the project's slowest era to build. Dual indexing solves the symbol search problem. Feature governance prevents half-shipped capabilities.

This is the layer that every software project has. The product. The code. The thing someone would pay for or depend on.

What makes perl-lsp unusual is that Layer 1 is not the only product.

---

## Layer 2: The Swarm OS

Layer 2 is the machinery that builds Layer 1. It is a development operating system for AI agents --- not a tool that was bolted on after the product was built, but infrastructure that co-evolved with the product and is versioned alongside it in the same repository.

The components:

| Component | Count | Purpose |
|-----------|-------|---------|
| Skills | 15 | Codified procedures agents invoke by name |
| Slash commands | 48 | Operator entrypoints for coordination |
| Hooks | 3 | Deterministic behavioral enforcement |
| CI gate tiers | 3 | PR-fast, merge gate, nightly |
| Coordinator roles | 5 | Scout, Builder, Reviewer, Ops, Improver |

Skills are the core mechanism. `/verify-build` began as three lines of cargo commands copy-pasted into every agent prompt. When agents started forgetting the `--tests` flag on clippy, the commands were extracted into a skill. When agents started skipping the format check, the skill was updated. Each failure mode was absorbed, making the failure impossible for future agents. The skill library grew from zero to 32 entries across five development cycles.

Hooks encode a specific lesson that the project learned the hard way: prompt instructions are suggestions; mechanical enforcement is the only reliable compliance mechanism. When Cycle 2 agents were prompted to write metrics entries, zero of 30 PRs complied. After hooks were added, compliance was immediate and permanent.

Worktree isolation provides true parallelism. Every builder agent works in its own git worktree --- a separate checkout with its own working tree and index. Combined with the microcrate architecture, this means 50--100 agents can build simultaneously with zero file conflicts. The architecture IS the parallelism enabler.

The scout-constrain-build pattern is the central innovation. Constrained tasks succeed at ~90%. Unconstrained tasks succeed at ~50%. That is not a marginal improvement. It is the difference between a system that works and one that wastes half its compute.

Layer 2 is unusual because most projects do not treat their development process as a product. They have CI pipelines and linters, but those are not the same as a structured, versioned, skill-based operating system for parallel agent development. The `.claude/` directory is not configuration. It is infrastructure.

---

## Layer 3: The Memory and Evidence System

Layer 3 is the knowledge that keeps Layers 1 and 2 legible. It is the institutional memory of the project --- not in anyone's head, but committed to the repository as versioned files that agents read automatically.

| Component | Count | Purpose |
|-----------|-------|---------|
| Memory files | 106+ | Persistent cross-session learnings |
| Research documents | 10+ | Deep analysis of specific problems |
| ADRs | 7 | Architectural Decision Records |
| Corpus receipts | per-session | Evidence of parser improvement |
| Articles | 10+ | Narrative explanations of methodology |

The memory system has four types:

**User memories** encode developer preferences and collaboration style. An agent that knows the developer has deep Rust expertise but is new to a specific subsystem will explain differently than one that does not.

**Feedback memories** capture what worked and what did not --- and why. "Rapid merges cancel each other's CI runs; wait for completion between batches" was discovered in Cycle 2 when five consecutive merges each cancelled the previous CI run. "Scouts must verify claims via `gh pr view --json state`, not just read issue descriptions" was discovered in Cycle 5 when a scout filed duplicate work because it trusted an issue that had already been fixed.

**Project memories** record current state, constraints, and deadlines. They decay fast, which is why each includes a "Why" and "How to apply" section so future agents can judge whether the memory is still load-bearing.

**Reference memories** point to where information lives in external systems --- which Linear project tracks pipeline bugs, which Grafana dashboard the oncall watches.

The evidence system is equally important. Metrics in perl-lsp are computed, not hand-edited. `CURRENT_STATUS.md` is auto-generated. The CPAN corpus parse rate is measured by the CI gate running the actual parser against actual CPAN modules. The number cannot be inflated by changing the test. Receipts --- structured outputs from builder agents showing which acceptance criteria are met, which tests were added, the actual `cargo test` output --- are how one human oversees 100 agents. You do not read 100 diffs. You read 100 receipts and spot-check the ones that look unusual.

Layer 3 is what most AI projects lack entirely. They generate code (Layer 1) and some have tooling (Layer 2), but almost none externalize their learning into a persistent, versioned, machine-readable knowledge base that makes every future session better than the last.

---

## Why Three Layers Matter

Most software projects have one layer. The product. The code. Everything else --- process knowledge, development methodology, institutional memory --- lives in people's heads, in Confluence pages that drift, in Slack threads that scroll past.

Some projects add a second layer. CI pipelines, custom linters, internal developer tools. These are valuable, but they are static. They enforce rules but do not learn from experience.

Almost no projects have a third layer --- a persistent, structured, versioned record of what the project has learned about how to build itself. The compound effect of having all three is the reason perl-lsp can run 100 agents in a session and produce 56 reviewed, tested, CI-gated PRs.

The compounding works like this:

**Layer 3 improves Layer 2.** When a session discovers that merge batches should be limited to 3 to avoid CI cascade cancellations, that discovery becomes a memory file. The next session's coordinator reads the memory and enforces the constraint without being told. Eventually, the lesson graduates from memory to a hook --- permanent enforcement. Knowledge flows upward from observation to memory to infrastructure.

**Layer 2 improves Layer 1.** Better skills, better scouts, better enforcement hooks produce higher-quality agent output. The `/parser-fix` skill encodes the exact TDD loop that produces successful parser fixes. An agent using it does not need to discover the loop independently. It just works.

**Layer 1 validates Layers 2 and 3.** The CPAN corpus parse rate is the ultimate test of whether the machinery and the memory are actually producing better software. If the parse rate is not improving, something in Layers 2 or 3 is wrong. The product is the ground truth.

The three layers form a feedback loop. The product generates evidence. The evidence updates the memory. The memory improves the machinery. The machinery builds a better product. Each cycle makes the next cycle faster.

---

## The Four Launch Columns

At any given moment, work in the repository maps to one of four columns:

| Column | Layer | Examples |
|--------|-------|---------|
| **Product** | 1 | Parser fixes, LSP features, diagnostics, completions, DAP |
| **Control Plane** | 2 | Skills, hooks, coordinator definitions, CI gates, worktree tooling |
| **Evidence** | 3 | Corpus receipts, computed metrics, test results, ratcheted baselines |
| **Narrative** | 3 | Memory files, research documents, articles, ADRs |

This framing explains why the commit history looks unusual. In a traditional project, the vast majority of commits are Product. In perl-lsp, a significant fraction of commits are Control Plane, Evidence, or Narrative. This is not overhead. It is the investment that makes the Product commits possible at scale.

A session might look like this:

1. Scout agents read the corpus failures and file issues (Evidence feeds Product)
2. Builder agents implement fixes in worktrees using skills (Control Plane builds Product)
3. Reviewer agents check the diffs against acceptance criteria (Control Plane validates Product)
4. CI gates verify the build and ratchet the baseline (Evidence validates Product)
5. Memory files capture what worked and what did not (Narrative records Evidence)
6. Skills are updated to absorb failure modes (Narrative improves Control Plane)

Every column feeds every other column. The system is circular by design.

---

## Quality Came Before Cheapness

A natural question: did the swarm OS (Layer 2) add quality to the project, or did it just add speed?

Neither. It made existing quality cheap to maintain.

perl-lsp already had review discipline before agents were running in parallel. It already had a zero-panic policy (no `unwrap()`, `expect()`, `panic!()`, or `todo!()` in production code). It already had CI gates, test suites, and coding standards. Era 3 --- the slowest era, with 8.9 commits per active day --- was spent building these foundations.

The swarm OS did not add quality. It encoded the quality standards that already existed as executable, enforceable infrastructure. The zero-panic policy was always a rule. The swarm OS turned it into a clippy lint that blocks merge. Review was always expected. The swarm OS turned it into a separate reviewer agent that cannot be skipped. Testing was always required. The swarm OS turned it into a `/verify-build` skill that runs `cargo fmt`, `cargo clippy`, and `cargo test` in sequence and will not proceed until all three pass.

This is a crucial distinction. A project that adopts swarm development without first having quality standards will scale its lack of quality. 100 agents generating unreviewed, untested code is 100x worse than one developer doing the same. The swarm OS is an amplifier. What it amplifies depends on what was already there.

The velocity paradox from the project's history makes this concrete:

| Era | Pace | Quality Investment |
|-----|------|--------------------|
| Era 3: Architectural | 8.9 commits/active day | 7 ADRs, mutation testing, microcrate extraction, corpus pipeline |
| Era 4: Copilot Fleet | 36.4 merged commits/day | Minimal --- relied on Era 3's infrastructure |
| Era 5: Structured Swarm | 56 PRs/session | Extended Era 3's infrastructure with skills, memory, hooks |

Era 3 was the investment. Era 5 was the return. The lesson: build the safety net before you add the trapeze.

---

## Implications

The three-layer structure is not specific to perl-lsp, Perl, Rust, or language servers. It is a pattern that any project can adopt.

**Layer 2 is buildable today.** Skills are markdown files with frontmatter. Hooks are shell scripts triggered by events. Worktrees are a git feature. CI gates are YAML. None of this requires custom infrastructure. The barrier is not technology. It is the decision to treat development process as a versioned product rather than tribal knowledge.

**Layer 3 is buildable today.** Memory files are markdown with frontmatter. An index file tracks them. Agents read them on startup. The format is simple enough that a team could adopt it in an afternoon. The discipline to maintain it is harder, but the format is not the barrier.

**The competitive moat is in the compound learning.** perl-lsp's 106 memory files encode five eras of operational lessons. A team starting today starts at zero. But the rate of accumulation is not linear --- it is proportional to the number of sessions, the number of agents, and the breadth of problems encountered. A team that starts building Layer 3 today will have 50 memory files in a month and 200 in a quarter. The knowledge compounds.

**The three layers explain the human role.** In a three-layer system, the human is not a code writer, a code reviewer, or even a project manager. The human is the architect of all three layers. Strategic direction for the product (Layer 1). Design decisions for the machinery (Layer 2). Curation of the knowledge base (Layer 3). This is a genuine role --- not a diminished one --- and it requires judgment that no agent currently has: which problems matter, which memories are still relevant, which architectural choices will enable the next era of scaling.

**You do not need 100 agents to benefit.** The three-layer pattern is valuable at any scale. A solo developer with 3 skills, 10 memory files, and a CI gate already has a three-layer product. The difference between 3 skills and 32 skills is scale, not kind. Start small. The layers compound on their own.

---

## Conclusion

perl-lsp looks unusual because it is three products in one repository. An LSP that users install. An operating system that agents use to build the LSP. A knowledge base that makes both of them better with every session.

The three layers co-evolved because separating them would break the feedback loop that makes each one valuable. The product generates evidence. The evidence updates the memory. The memory improves the machinery. The machinery builds a better product. Separating any layer from the others would interrupt this cycle.

The insight is not that perl-lsp has good tooling or good documentation. The insight is that the tooling, the documentation, the memory, the skills, the hooks, the receipts, and the ratchets are all versioned infrastructure that improves with every session --- and that this improvement is the actual product, more than any single parser fix or LSP feature.

The repo is improving the machinery that improves the code, and improving the memory that explains both. That is what makes it three products, not one.

---

*perl-lsp is an open-source Perl Language Server. The three-layer structure described in this article is observable in the repository's `.claude/` directory (Layer 2), `docs/` and memory files (Layer 3), and `crates/` (Layer 1). All numbers are drawn from the git history and committed artifacts as of March 2026.*

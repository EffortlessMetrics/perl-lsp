# Q3 Control-Plane Archaeology
## How `agents4` Turned The Canonical Q3 Swarm Into A Real Operating Surface

The repo already has a note proving that `agents4` is the canonical Q3 swarm.
This note goes one layer deeper: what kind of control plane `agents4` actually
was.

The answer is more developed than "a lot of agent files." `agents4` is already
an explicit, phase-aware operating surface with:

- three named delivery phases
- worktree-serial write discipline
- GitHub-native receipts
- a single authoritative Ledger comment
- gate evolution across phases
- perl-lsp-specific validation and evidence formats

That is why Q3 looks historically important. The repo was already trying to
industrialize delivery, not just parallelize prompts.

---

## 1. `agents4` Is The Canonical Q3 Surface

The local git history is direct here.

Commit `104bdc17e` on `2025-09-23` adds the full `agents4` pack, including:

- `issue-to-draft.md`
- `draft-to-pr.md`
- `pr-to-merge.md`
- the `generative/`, `review/`, and `integration/` role directories

Those are the two naming schemes the repo now uses for the same Q3 swarm:

- role packs: `generative`, `review`, `integration`
- delivery path: `issue-to-draft`, `draft-to-pr`, `pr-to-merge`

That is why `agents4` matters. It is not a later reconstruction. It is a
preserved operating surface from the period itself.

---

## 2. The Q3 Swarm Was Already Phase-Aware

`agents4` does not treat the swarm as one flat loop.

[`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
defines Generative as the first phase, explicitly feeding Review.

[`.claude/agents4/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/draft-to-pr.md)
defines Review as inheriting Generative baselines and promoting work toward
Integrative.

[`.claude/agents4/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/pr-to-merge.md)
defines Integrative as the final production-readiness layer before merge.

The three phases are not rhetorical. They change what counts as proof:

- Generative establishes baseline implementation and benchmark evidence
- Review validates deltas and readiness
- Integrative validates production constraints and mergeability

That is already a real pipeline model, not just "generate, then look at it."

---

## 3. `agents4` Already Had Gate Evolution

One of the most revealing details in `agents4` is that the gates are not static
across phases.

The flow docs explicitly evolve the proof burden:

- Generative emphasizes `benchmarks` as baseline-establishing evidence
- Review inherits that baseline and adds `perf`
- Integrative inherits the prior evidence and adds final readiness validation

In the perl-lsp specialization, this becomes even more specific:

- parser validation
- LSP protocol validation
- highlight validation
- incremental parsing performance
- adaptive threading for LSP-heavy tests

That tells us the Q3 swarm already understood something important: the evidence
required to write code is not the same as the evidence required to merge code.

---

## 4. The Pack Was Specialized For Perl-LSP Reality

`agents4` is not just donor structure with names changed.

Its commands and evidence formats are tailored to this repo:

- `cargo test -p perl-parser`
- `cargo test -p perl-lsp`
- `cd xtask && cargo run highlight`
- parser and LSP-specific evidence summaries
- adaptive `RUST_TEST_THREADS=2`
- explicit parsing and workspace-navigation claims in the receipt format

The generative customizer makes the intent even clearer:

[`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md)
targets Perl LSP's GitHub-native Issue to PR Ledger workflow and parser
validation patterns, not a generic software project.

So while `agents3` preserves a donor layer, `agents4` is the point where the
repo makes the swarm canonically its own.

---

## 5. The Core Discipline Was Already Present

The Q3 control plane already carries the rules later treated as hallmarks of
the current swarm:

- work in `worktree-serial mode`
- no local run IDs or git tags
- traceability through commits, Check Runs, and the Ledger
- edit the single authoritative Ledger comment in place
- keep status in checks, not in comment spam

Those rules appear in all three flow files.

That means the repo's later emphasis on:

- receipts over summaries
- worktree isolation
- one-writer boundaries
- durable state

is an extension of Q3 discipline, not a total reinvention.

---

## 6. The Role Packs Show Early Specialist Thinking

The directory layout matters almost as much as the flow files.

Under `agents4`, each phase already has a specialist roster:

- Generative has builders, test creators, mutation testers, fuzz testers,
  schema validators, and publication prep roles
- Review has freshness, architecture, contract, coverage, docs, perf, and
  hardening specialists
- Integration has rebase, feature-matrix, security, benchmark, cleanup,
  merge-prep, and merger specialists

That is the Q3 swarm already decomposing SDLC work into domain-specific roles.

The repo later changes how those roles are surfaced, archived, and reused. But
the specialization instinct is already here.

---

## 7. Why `agents4` Matters Historically

The repo's Q3 history is often summarized as:

- massive PR waves
- three phases
- heavy review and integration load

All of that is true. But `agents4` shows the more important thing:

the repo already had a reasonably serious control-plane model for how that work
should move.

What later eras add is not the basic idea of orchestration. They add:

- more durable state
- cleaner separation between commands and skills
- hooks for deterministic enforcement
- a better answer to memory, queue state, and cross-session reuse

That is why `agents4` is best read as the canonical Q3 operating surface, not
just a historical curiosity.

---

## Evidence Pointers

- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
- [`.claude/agents4/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/draft-to-pr.md)
- [`.claude/agents4/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/pr-to-merge.md)
- [`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md)
- [Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- commit `104bdc17e`

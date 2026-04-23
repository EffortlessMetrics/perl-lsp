# Trusted Change Archaeology
## How This Repository Industrialized Trust

This note tracks a specific shift in the repository's operating model: from manual trust in a reviewer or maintainer to mechanical trust built from receipts, catalogs, validation lanes, and durable swarm state.

The repo did not become trustworthy by asking humans to read more diffs. It became trustworthy by making claims cheap to verify and expensive to fake.

---

## 1. The Trust Contract Is Explicit

The project docs state the contract plainly:

- `docs/project/AGENTIC_DEV.md` says the repo is AI-native, not AI-assisted
- `docs/project/CURRENT_STATUS.md` says numeric claims must come from evidence sources
- `docs/project/METRICS_PROVENANCE.md` defines provenance fields for every metric
- `docs/reference/FORENSICS_SCHEMA.md` defines a dossier model centered on decisions + proof

That is the philosophical base layer. The practical consequence is simple:

1. claims must be sourced
2. claims must be reproducible
3. claims must survive mechanical checks

The repo is not trying to be "careful" in a human sense. It is trying to be falsifiable.

---

## 2. The First Failure Mode Was Claim Drift

`docs/project/LESSONS.md` is the strongest evidence that trust was industrialized in response to real mistakes.

The earliest entries are not code bugs. They are documentation and metric failures:

- coverage was overstated in `ROADMAP.md` and `CLAUDE.md`
- performance claims were published before benchmark receipts existed
- superlatives outpaced evidence
- issue IDs were accidentally treated like PR IDs

The fixes all point the same way:

- compute the numbers from source files
- link claims to receipts
- separate issue numbers from PR numbers
- make drift fail locally with `just status-check`

That is the moment the repo stops trusting prose and starts trusting computed state.

---

## 3. Receipts Replace Faith

The repo's trust model is receipt-first:

- `docs/project/AGENTIC_DEV.md` says receipts prove claims
- `docs/project/CURRENT_STATUS.md` makes `just ci-gate` the merge receipt
- `docs/project/METRICS_PROVENANCE.md` requires `value`, `kind`, `basis`, `coverage`, and `confidence`
- `docs/reference/FORENSICS_SCHEMA.md` requires proof bundles, drift reports, and next prevention actions

This matters because it changes the unit of trust.

The human no longer has to remember whether a change was verified. The artifact has to carry the proof.

That is a very different workflow from manual code review:

- manual review asks, "Do I believe this?"
- receipt-based review asks, "What evidence exists?"

The second is weaker emotionally and stronger operationally.

---

## 4. Validation Became Layered

The CI docs show that trust was not centralized in one gate. It was decomposed into lanes:

- `just ci-gate` for the fast merge gate
- `just ci-full` for deeper validation
- label-gated test lanes for stress, security, extras, mutation, property, and coverage
- `nix develop -c just ci-gate` as the canonical local gate

`docs/project/CI_TEST_LANES.md` is the clearest statement of this model:

- core and LSP tests run by default
- stress and security are label-gated
- property and mutation are separate lanes
- concurrency cancellation and path filters control CI spend

This is trust industrialized as a budgeted system.

The repo is not saying "run all the tests every time." It is saying "run the right tests in the right lane, and make the lane itself part of the contract."

---

## 5. Mutation And Fuzz Testing Changed The Meaning Of Confidence

The parser and testing docs show that the repo moved beyond pass/fail tests into test-quality verification:

- `docs/reference/MUTATION_TESTING_METHODOLOGY.md` documents an 87% mutation score and mutation-survivor analysis
- `docs/project/PARSER_EVOLUTION.md` treats mutation testing and fuzzing as routine nightly validation
- `docs/project/CI_TEST_LANES.md` explicitly separates property, mutation, stress, and extras lanes
- `.claude/agents4/issue-to-draft.md` includes canonical gates for `mutation`, `fuzz`, and `security`

That combination matters because it changes what "tested" means.

The repo no longer trusts a test suite just because it passes. It asks whether the test suite kills bad mutations, survives edge cases, and exercises the right risk surface.

That is a stronger kind of trust. It is also more expensive, which is why it had to become mechanical.

---

## 6. Review Became A Machine, Not A Mood

The current control plane encodes review as a workflow, not a judgment call:

- `.claude/commands/review-pr.md` enforces one PR per review agent
- `.claude/commands/pr-ready.md` turns review completion into a readiness transition
- `.claude/skills/triage-prs/SKILL.md` clusters duplicates and keeps the best PR
- `.claude/swarm-state/README.md` treats findings, pitfalls, and queue state as durable memory

This is a major shift from manual trust.

Instead of asking a person to remember which PR is ready, which one is stale, and which one has already been tried, the repo stores that state in tracked surfaces and pushes the decision into named operations:

- review
- ready
- triage
- queue
- findings

The result is that review stops being an emotional bottleneck and becomes a routable process.

---

## 7. Swarm-State Is Institutional Memory

The `swarm-state` directory is the repo's durable memory layer.

The README makes the intended use explicit:

- `discovered-issues.md` for leads
- `known-pitfalls.md` for reusable traps
- `completed-slices.md` for dedup and lifecycle status
- `swarm-queue.json` for active overlap
- `findings.json` for stable control-plane conclusions

That structure is important because it separates:

- transient observations
- reusable lessons
- queue bookkeeping
- durable doctrine

`known-pitfalls.md` is especially telling: it is append-only during swarm operation. The repo expects to learn from failure and preserve the lesson in a file the next agent can read.

That is not just documentation. That is operational memory.

---

## 8. Xtask Turned Validation Into A Single Tooling Surface

`docs/project/XTASK_MIGRATION.md` shows the same theme in tooling form.

The migration replaces scattered shell and Python scripts with Rust `xtask` commands where it matters:

- `cargo xtask gates` subsumes gate execution
- `cargo xtask features verify` covers feature invariants
- `cargo xtask publish-crates` covers publishing
- `cargo xtask test-lsp` and `cargo xtask bench` cover repeatable validation

The point is not just convenience. It is consistency.

By moving validation and release logic into Rust, the repo reduces the number of places where trust can drift:

- fewer one-off scripts
- fewer hand-maintained invocation paths
- more typed, testable, workspace-aware automation

That is industrial trust engineering, not just cleanup.

---

## 9. The Historical Arc

The repo's trust arc looks like this:

1. trust the maintainer's judgment
2. add receipts and provenance
3. make docs and metrics fail on drift
4. split validation into lanes
5. add mutation, fuzz, and property-test confidence
6. encode review, readiness, and triage as control-plane operations
7. persist pitfalls and findings in tracked swarm state
8. move recurring validation logic into `xtask`

The important transition is that trust becomes a property of the system, not of the conversation.

That is what makes the codebase unusually interesting as an AI-age artifact. It is not just using agents to write more code. It is building a machine that can justify its own output.

---

## Evidence Pointers

- [docs/project/LESSONS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/LESSONS.md)
- [docs/project/AGENTIC_DEV.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [docs/project/METRICS_PROVENANCE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/METRICS_PROVENANCE.md)
- [docs/reference/FORENSICS_SCHEMA.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/reference/FORENSICS_SCHEMA.md)
- [docs/project/CI_TEST_LANES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_TEST_LANES.md)
- [docs/project/XTASK_MIGRATION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/XTASK_MIGRATION.md)
- [docs/project/CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [`.claude/commands/review-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/review-pr.md)
- [`.claude/commands/pr-ready.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/commands/pr-ready.md)
- [`.claude/skills/triage-prs/SKILL.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/skills/triage-prs/SKILL.md)
- [`.claude/swarm-state/README.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/README.md)
- [`.claude/swarm-state/known-pitfalls.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/swarm-state/known-pitfalls.md)
- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)

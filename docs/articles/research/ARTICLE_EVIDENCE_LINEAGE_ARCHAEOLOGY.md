# Article Evidence Lineage Archaeology
## How Launch Claims Stay Source-Linked

This note is a source map for future article writing. It does not try to restate
the launch story again. It records which article claims are backed by which
exact issue/PR/doc combinations, so future prose can stay pinned to evidence
instead of drifting into nice-sounding summary.

The useful rule is simple:

- a claim about workflow should point to a workflow doc plus one concrete PR
- a claim about parser behavior should point to issue/PR lineage plus a test or
  corpus receipt
- a claim about trust or receipts should point to the receipt chain, not just a
  talk or a slogan
- a claim about article-worthiness should point to the issue/PR/doc chain that
  makes it recoverable later

All examples below were verified from the repo and GitHub archive on
`2026-03-19`.

---

## 1. Swarm Claims Need Two Kinds Of Evidence

Claims about the swarm are strongest when they have both:

- a descriptive history note
- a concrete control-plane or PR example

The repo already splits those responsibilities cleanly.

Use these pairings:

- [`DEVELOPMENT_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/DEVELOPMENT_ARCHAEOLOGY.md) for scale, crate growth, parser lineage, and agent counts
- [`ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md) for the five-era sequence and velocity changes
- [`Q3_SWARM_TALK_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_TALK_ARCHAEOLOGY.md) for the theory of trusted change, receipts, and flows
- [`CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md) for `.claude` / `.jules` lineage and current surfaces
- [`WORKTREE_PARALLELISM_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/WORKTREE_PARALLELISM_ARCHAEOLOGY.md) for the move from lane ideas to worktree execution

The concrete evidence chain for a claim like “the repo became PR-shaped before
it became fully industrialized” is:

1. [`Q3_SWARM_PR_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md) for the late-September PR-heavy shift
2. [`REVIEW_LABEL_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md) for the label state machine
3. [`MERGECODE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGECODE_ARCHAEOLOGY.md) for the earlier `issue-to-draft` / `draft-to-pr` / `pr-to-merge` doctrine layer
4. [`MERGE_DISCIPLINE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/MERGE_DISCIPLINE_ARCHAEOLOGY.md) for later control-plane hardening

That gives the claim a timeline, a control surface, and a workflow shape.

---

## 2. Parser-Fix Claims Need Issue, PR, And Learning Evidence

Parser claims are only trustworthy if they carry the full lineage, because a
single PR usually hides the actual discovery path.

The best recurring evidence chains are:

- `#1700` -> [`PR #2040`](https://github.com/EffortlessMetrics/perl-lsp/pull/2040) -> [`issue #2190`](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- `#1703` -> [`PR #2180`](https://github.com/EffortlessMetrics/perl-lsp/pull/2180) -> [`issue #2191`](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
- `#1889` -> [`PR #2039`](https://github.com/EffortlessMetrics/perl-lsp/pull/2039) -> [`issue #2195`](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)

Those chains support different article claims:

- “parser fixes are family trees, not isolated patches” is backed by [`ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md)
- “scouts need concrete failing snippets, not just bucket counts” is backed by the `#1700` learning issue
- “corpus ratchets become launch evidence” is backed by `#1889 -> #2039 -> #2195`

The useful source discipline here is:

1. issue for the problem identity
2. PR for the implementation identity
3. learning issue for the lesson identity
4. article issue for the publication identity

If a paragraph about parser work lacks one of those, it is probably too soft.

---

## 3. Receipts And Trust Need The Full Contract Chain

The strongest chain for receipt/trust claims is not just a PR body. It is:

1. PR `#209` as the original receipt-heavy scar story
2. PR `#274` as the template normalization step
3. issue `#210` as the governance request
4. [`.ci/receipt.schema.json`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json) as the machine-readable contract
5. [`xtask/src/tasks/gates.rs`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs) as the typed runner
6. [`scripts/run-gates.sh`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/scripts/run-gates.sh) as the older shell bridge
7. [`docs/forensics/prompts/measurement-auditor.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/forensics/prompts/measurement-auditor.md) as the audit surface

The claim chain this supports is:

- “receipts are not vibes” -> [`AGENTIC_DEV.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- “receipts can still lie” -> [`RECEIPTS_LIE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
- “proof moved from narrative to contract” -> `#209` + `#274` + `#210` + schema/runner

If an article makes a trust claim, it should cite at least one of those
surfaces directly.

---

## 4. Q3 Claims Need The Right Naming Scheme

The repo has two naming schemes for the same Q3 three-phase swarm:

- `generative` = `issue-to-draft`
- `review` = `draft-to-pr`
- `integration` = `pr-to-merge`

The evidence chain for that claim is:

1. [`Q3_SWARM_PR_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
2. [`REVIEW_LABEL_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/REVIEW_LABEL_ARCHAEOLOGY.md)
3. [`CONTROL_PLANE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)

That matters because launch prose can easily collapse the two naming schemes
into one. The repo history says they are the same three phases, just named
differently by layer.

---

## 5. Install And Readiness Claims Need Separate Evidence

The March 2026 scout residue was useful because it shows how easy it is to
overstate readiness or installation clarity if those claims are left as prose.

Use the current repo docs as the actual source chain:

- [`CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md) for evidence-backed readiness claims
- [`ROADMAP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md) for what is targeted vs shipped
- [`INSTALLATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/INSTALLATION.md) and [`EDITOR_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/how-to/EDITOR_SETUP.md) for user-facing install/setup flow
- [`docs/EDITORS/NEOVIM_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/NEOVIM_SETUP.md) and [`docs/EDITORS/HELIX_SETUP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/EDITORS/HELIX_SETUP.md) for editor-specific deep dives

For launch articles, the rule is:

- readiness claims belong in [`CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- planning claims belong in [`ROADMAP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md)
- article claims about install or first-run behavior need direct file/doc evidence, not just a scout summary

That keeps the article series from turning current product posture into historical
memory.

---

## 6. What To Cite In Future Articles

When writing the launch pieces, use this shape:

- swarm evolution claims -> `DEVELOPMENT_ARCHAEOLOGY.md` + `ERA_TIMELINE.md` + a control-plane note
- trust/receipts claims -> `RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md` + `RECEIPTS_LIE_ARCHAEOLOGY.md` + `GATE_RECEIPT_FORENSICS_ARCHAEOLOGY.md`
- parser-fix claims -> `ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md` + the issue/PR chain + the learning issue
- article-worthy historical claims -> `ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md` + the specific issue/PR trail that made the story recoverable

That is the practical standard: every strong paragraph should have a source
chain that another maintainer can follow back to the archive.

---

## Evidence Pointers

- [`DEVELOPMENT_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/DEVELOPMENT_ARCHAEOLOGY.md)
- [`ERA_TIMELINE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA_TIMELINE.md)
- [`Q3_SWARM_TALK_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_TALK_ARCHAEOLOGY.md)
- [`RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md)
- [`ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_FAMILY_GENEALOGY_ARCHAEOLOGY.md)
- [`TRUSTED_CHANGE_ARCHAEOLOGY.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/TRUSTED_CHANGE_ARCHAEOLOGY.md)
- [`CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)

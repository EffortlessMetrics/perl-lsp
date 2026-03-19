# MergeCode Roots Archaeology
## How `agents3` Preserves A Donor Control Plane Before The Canonical Q3 Swarm

This note separates two related historical claims:

1. what the committed repo proves directly
2. what requires maintainer context beyond the repo

The committed history proves that `.claude/agents3` is not a native perl-lsp
pack. It is a donor or transitional control-plane layer with explicit
MergeCode vocabulary, later specialized into the canonical perl-lsp Q3 swarm in
`agents4`.

That is strong enough to document without overclaiming off-repo history.

---

## 1. What The Repo Proves Directly

The strongest signal is simple: `agents3` still speaks in a vocabulary that is
not this repository's vocabulary.

Examples from committed files:

- [`.claude/agents3/generative/spec-creator.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/generative/spec-creator.md)
  says it transforms "MergeCode feature requirements"
- the same file targets the semantic-analysis pipeline
  `Parse -> Analyze -> Graph -> Output`
- the same file names `mergecode-core`, `mergecode-cli`, and `code-graph`
- [`.claude/agents3/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/pr-to-merge.md)
  refers to "MergeCode quality compliance"

That is stronger than a vague resemblance. It is donor-repo language embedded
directly in the preserved control plane.

---

## 2. The Donor Layer Already Carried Core Swarm Ideas

The donor pack is historically important because it already contains operating
ideas that later become native, more explicit, and more durable elsewhere in
the repo.

[`.claude/agents3/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/issue-to-draft.md)
is the clearest evidence:

- work in `worktree-serial mode`
- traceability is `commits + Check Runs + the Ledger`
- gate Check Runs must be mirrored into the Ledger
- edit the single authoritative Ledger comment in place
- use progress comments for narrative rather than status spam

The same structure appears in:

- [`.claude/agents3/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/draft-to-pr.md)
- [`.claude/agents3/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/pr-to-merge.md)
- [`.claude/agents3/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/agent-customizer-generative.md)

So the repo's later emphasis on worktree discipline, GitHub-native receipts,
and single-source flow state did not appear from nowhere in March 2026. Those
ideas were already present inside the donor layer.

---

## 3. The Transplant Was Adaptation, Not Reinvention

The best comparison is `agents3` versus `agents4`.

The flow skeleton is clearly the same:

- `issue-to-draft`
- `draft-to-pr`
- `pr-to-merge`
- GitHub-native receipts
- Ledger anchors
- worktree-serial discipline

But the specialization changes.

`agents3` uses donor language like `mergecode-core`, `code-graph`, and
`docs/explanation/`. `agents4` rewrites the same structure for perl-lsp
reality:

- parser and LSP-specific validation
- perl-lsp repo commands
- perl-lsp evidence surfaces
- perl-lsp GitHub-native Issue to PR workflow language

This is especially clear in
[`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md),
which explicitly describes adapting generic agents to "Perl LSP's
GitHub-native Issue→PR Ledger workflow."

The historical reading is straightforward:

- `agents3` is the donor or transitional pack
- `agents4` is the canonical perl-lsp specialization

---

## 4. There Was A Prehistory Before The Donor Pack

The repo was already experimenting with orchestration before the donor layer
landed.

[`.claude/ORCHESTRATION_GUIDE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/ORCHESTRATION_GUIDE.md),
added in commit `3341bebdb` on `2025-08-28`, already contains:

- orchestration guidance
- GH CLI usage
- routing ideas
- review loops

What it does not yet have is the full formalism later visible in `agents3`:

- namespaced gate checks
- single authoritative Ledger comment
- explicit flow guards
- the full three-phase issue-to-draft / draft-to-pr / pr-to-merge model

That suggests a two-step history:

- August 2025: local orchestration ideas exist
- late September 2025: a more formal donor control plane lands

---

## 5. The Late-September Import Looks Composite

The late-September history is not a clean single-origin story.

Date clues from local git history:

- `104bdc17e` on `2025-09-23` adds a large donor bundle under
  `.claude/agents3 - to update/*`
- `e62fe5700` on `2025-09-23` normalizes that path into `.claude/agents3/*`
- `0d7f3d757` on `2025-09-23` is titled
  `Refactor agent-customizer for BitNet.rs to Perl LSP`
- `6e3bbcdf3` on `2025-09-23` updates agent-customizer docs for Perl LSP
  integration
- `2e5a58bfa` on `2025-09-24` is the large PR `#159` wave that heavily modifies
  the packs

That means the Q3 control plane is best understood as a composite transplant
lineage:

- one strand preserved directly in `agents3` with MergeCode vocabulary
- another strand visible in perl-lsp specialization work on `agents4`
- at least one nearby adaptation step explicitly tied to BitNet.rs

The important point is not origin purity. It is that the repo was already
industrializing by transplanting and specializing working control-plane ideas.

---

## 6. What Survived Into Later Swarms

Even after later surfaces became more mature, the donor layer's core ideas
survived:

- worktrees as the write boundary
- receipts over chat transcripts
- specialized lanes instead of one generic agent
- GitHub-native traceability
- single-source state rather than scattered status comments

Later eras do not discard those ideas. They externalize them further into:

- commands
- skills
- hooks
- `swarm-state`
- deterministic `worktree-agent-*` execution

Seen that way, `agents3` matters not as dead archive material but as a fossil
layer showing where later swarm doctrine came from.

---

## 7. Maintainer Context And Historical Limits

Maintainer context says MergeCode was an internal context packer and AST tooling
environment, and that the swarm was designed there before being specialized
here.

That context fits the committed evidence well. But the repo alone proves a
narrower statement:

- perl-lsp preserves a clearly MergeCode-derived donor control plane in
  `agents3`
- that donor layer was later specialized into the canonical perl-lsp swarm in
  `agents4`

That narrower claim is the one this note treats as fully evidenced from the
committed repo.

---

## Evidence Pointers

- [`.claude/ORCHESTRATION_GUIDE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/ORCHESTRATION_GUIDE.md)
- [`.claude/agents3/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/issue-to-draft.md)
- [`.claude/agents3/draft-to-pr.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/draft-to-pr.md)
- [`.claude/agents3/pr-to-merge.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/pr-to-merge.md)
- [`.claude/agents3/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/agent-customizer-generative.md)
- [`.claude/agents3/generative/spec-creator.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/generative/spec-creator.md)
- [`.claude/agents4/issue-to-draft.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/issue-to-draft.md)
- [`.claude/agents4/agent-customizer-generative.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents4/agent-customizer-generative.md)
- commits `3341bebdb`, `104bdc17e`, `e62fe5700`, `0d7f3d757`, `6e3bbcdf3`,
  `2e5a58bfa`

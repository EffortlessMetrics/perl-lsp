# MergeCode Archaeology
## How `agents2` and `agents3` Turned Prompts Into A GitHub-Native Delivery Layer

This note treats `agents2` and `agents3` as a distinct historical layer, not
just an early prompt pack. The important change is that the repo starts
encoding delivery as receipts, ledgers, and flow boundaries instead of as
chatty one-off instructions.

Maintainer context matters here: MergeCode itself was an internal context
packer and AST-tooling environment, and the swarm was designed there before its
surfaces were imported and specialized for `perl-lsp`. The committed repo does
not preserve the whole off-repo MergeCode system, but it does preserve the
doctrine layer that was transplanted from it.

---

## 1. `agents2` Was The MergeCode Doctrine Pack

The `agents2` tree is the broad MergeCode-era surface. It already separates the
work into named lanes:

- `generative/`
- `integration/`
- `review/`
- plus the older `mantle/` and `other/` variants that show the same ideas in
  slightly different packaging

That structure matters. It is not a single generic assistant prompt. It is a
workspaced doctrine for adapting agents to this repo's Rust parser, workspace,
and release constraints.

The strongest evidence-backed framing is therefore narrower than "MergeCode is
fully present in the repo" and stronger than "these are just some old prompts."
What the repo actually preserves is a transplanted doctrinal layer from the
internal MergeCode environment where the swarm had already been designed.

The root `agent-customizer.md` makes that explicit: generic agents are supposed
to be specialized for the parser ecosystem, with repo-specific performance,
security, testing, and workspace expectations. In git history, this layer is
born in the large `agents2` import and refactor wave around the March 2026
MergeCode work, then retained as archived history rather than erased.

---

## 2. `agents3` Makes The Three Flows Explicit

`agents3` is the doctrinal step where the repo stops talking in broad lanes and
starts naming the flows:

- `issue-to-draft`
- `draft-to-pr`
- `pr-to-merge`

This is the same underlying system, but now the control plane is visible:

- work in **worktree-serial mode**
- no local run IDs or git tags
- traceability comes from commits + Check Runs + the Ledger
- after non-trivial changes, emit a gate Check Run and mirror it in the Ledger
- edit the **single authoritative Ledger comment** in place
- use progress comments for narrative, not status spam

The result is GitHub-native receipts instead of local ceremony. The issue ledger
is migrated into the PR ledger, and the PR becomes the canonical record of
receipt, routing, and gate state.

---

## 3. Why This Layer Matters

This is the bridge between the older prompt-pack era and the later swarm
control plane.

`agents2` shows the repo learning how to shape agents for its own shape:
customization, review lanes, integration lanes, parser-specific validation,
and MergeCode-style receipts. `agents3` turns that into a real operating
protocol with a single ledger, explicit gate naming, and serial worktree
discipline.

That matters because the later `.claude/agents`, `.claude/commands`,
`.claude/skills`, `.claude/hooks`, and `.claude/swarm-state` surfaces did not
appear from nowhere. They are the next step after this layer:

- first, make the agents repo-aware
- then, make the flows explicit
- then, make the receipts GitHub-native
- then, make the memory durable

Seen that way, MergeCode is the doctrine layer that made the later swarm
infrastructure legible.

---

## Evidence Pointers

- [.claude/agents2/agent-customizer.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents2/agent-customizer.md)
- [.claude/agents3/issue-to-draft.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/issue-to-draft.md)
- [.claude/agents3/draft-to-pr.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/draft-to-pr.md)
- [.claude/agents3/pr-to-merge.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.claude/agents3/pr-to-merge.md)
- [docs/project/AGENTIC_DEV.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEV.md)
- [docs/project/AGENTIC_DEVELOPMENT.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md)

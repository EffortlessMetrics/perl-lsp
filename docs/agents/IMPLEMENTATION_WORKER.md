# IMPLEMENTATION_WORKER.md - Worker Operating Manual

This manual applies to a bounded implementation, review, proof, triage, or cleanup
assignment. The root [`AGENTS.md`](../../AGENTS.md) routes parent and worker roles;
this document contains only worker procedure and project-wide implementation rules.

## Worker boundary

- Execute one supplied issue, PR, spec, or action packet.
- Stay inside the declared read scope, write cage, branch, worktree, and proof obligations.
- Do not select unrelated work, change portfolio priority, or recursively delegate.
- Return when the objective is answered, the scope is crossed, a premise is contradicted,
  or required proof is unavailable.
- Preserve uncertainty as `NOT_PROVEN`; do not turn missing or instrument-failed proof
  into success.

The issue, linked spec, branch, worktree, PR, checks, reviews, and receipts are the
authorities for the assignment. Conversation is a handoff aid, not a substitute for
those artifacts.

## Before starting

Check the current base and assignment rather than relying on an old handoff:

```bash
git log origin/main --oneline -20
git status --short --branch
gh issue view <issue> --json title,state,body,url
gh pr view <pr> --json headRefOid,baseRefOid,state,url
```

Read only the issue, accepted spec or `.spec/` builder view, package-local
instructions, and relevant source or proof surfaces named by the packet. Do not load
unrelated roadmap, history, or orchestration material into a bounded worker context.

## Project shape

| Path | Purpose |
| --- | --- |
| `crates/perl-lsp-rs/` | LSP binary and server host |
| `crates/perl-dap/` | Debug Adapter Protocol server |
| `crates/perl-parser/` | Native recursive-descent parser |
| `crates/perl-lexer/` | Context-aware tokenizer |
| `crates/perl-parser-core/` | Shared parser infrastructure |
| `crates/perl-semantic-analyzer/` | Semantic analysis and resolution |
| `crates/perl-workspace-index/` | Cross-file indexing and lookup |

Package-local instruction files remain the domain ownership context for the package
being changed. Do not turn them into portfolio or session-state stores.

## Scoping and ownership

- Touch one concern: one fix, feature, refactor, documentation seam, or proof slice.
- Do not bundle unrelated cleanup or rewrite files outside the edit cage.
- One accountable writer owns a branch and worktree at a time.
- Reviewers are read-only unless the packet explicitly grants a bounded repair write;
  any new head requires fresh review and proof.
- Do not weaken, drop, ignore, or comment out existing tests.
- Do not close, merge, retarget, or otherwise change a PR outside the supplied scope.

## Commit and PR conventions

Use one focused commit when the packet asks for a commit. Use the real issue number in
the title:

```text
type(scope): description (#NNNN)
```

If the issue identity is unavailable, stop and return to the parent for clarification;
do not invent a reference or use a placeholder. Keep the PR body concise and reviewer-
accessible: problem, fix, verification, claim boundary, and known gaps.

## Rust and test quality

Production code and tests must use fallible paths. Do not add or retain casual
`unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()`,
`println!()`, or `eprintln!()` in library code. Tests should return `Result<()>` or
use the repository's fallible test helpers. Every lint exception needs a reason.

Prefer `.first()`, checked access, `Result`/`Option`, useful error context, and
deterministic ordering. Do not hold locks across `.await`.

## Build and storage discipline

Use repository wrappers and scoped proof. Prefer:

```bash
just agent-check
just agent-test
just agent-clippy
just agent-pr-fast
```

For a single crate, use `scripts/cargo-safe` with the `agent` profile and `--locked`.
Avoid workspace-wide builds unless the packet explicitly assigns that proof. Run:

```bash
scripts/storage-doctor
```

before handoff when the assignment creates build output. Do not use `git stash`; use
`git restore <file>` for discarded edits or a focused work-in-progress commit when
the packet requires preserving partial work.

## Verification and handoff

Run the cheapest proof that establishes the packet's claim, then report:

- changed files and the exact head;
- commands and pass/fail/not-run results;
- what the proof establishes and does not establish;
- findings, uncertainty, and unresolved contradictions;
- durable artifact or receipt references;
- the next recommendation or explicit return condition.

For parser, path, string, numeric, or external-contract changes, include the edge or
oracle case that makes the seam trustworthy. A green wrapper command without selected
case or exact-head evidence is insufficient.

## Platform notes

CI runs on Linux while contributors may use Windows. Preserve CRLF/LF compatibility,
use literal paths when needed, and report wrapper or file-lock failures honestly. If a
required command is unavailable, use the nearest repository-local proof and mark the
gap rather than claiming completion.

## Further reading

- [`CLAUDE.md`](../../CLAUDE.md) for stable repository invariants and authority links
- [`docs/reference/COMMANDS_REFERENCE.md`](../reference/COMMANDS_REFERENCE.md) for commands
- [`docs/reference/WORKTREE_PROTOCOL.md`](../reference/WORKTREE_PROTOCOL.md) for isolation
- [`docs/agents/BUILDER_BRIEF_TEMPLATE.md`](BUILDER_BRIEF_TEMPLATE.md) for bounded packets
- [`docs/agents/EVIDENCE_STANDARD.md`](EVIDENCE_STANDARD.md) for claim-sized proof

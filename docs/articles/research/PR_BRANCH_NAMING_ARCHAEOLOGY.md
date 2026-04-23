# PR Branch Naming Archaeology
## How Head Branches And PR Titles Track The Repo's Workflow Shifts

This note uses GitHub PR metadata, not just commit history, because branch names and PR titles expose the operating model more clearly than commit counts do.

In the PR archive I pulled, `codex/` is still the largest head-branch family by a wide margin, but the other names show the real story: direct early work, a PR-shaped Q3 swarm, January concern lanes, a Copilot/release wave, and the current `worktree-agent-*` docs-and-control-plane pattern.

## 1. Early Work Still Reads Direct

The earliest PRs on `2025-08-26` are mostly `codex/<verb>-<noun>` branches with imperative titles:

- `codex/update-scope_analyzer-with-context-checks` -> `Handle hash subscript barewords`
- `codex/modify-package-handling-in-completion.rs` -> `Add package and path aware completions`
- `codex/extend-parser-rule-for-replacements` -> `Parse substitution replacement and modifiers`
- `codex/implement-incremental-parsing-and-tests` -> `Implement line/column mapping and integrate lexer for incremental parsing`
- `cleanup/post-pr1-fixes` -> `Cleanup after PR #1`

This is agent-assisted work, but the naming is still implementation-first. The branch tells you the task; the PR title tells you the deliverable. The workflow is not yet naming itself as a swarm.

## 2. Q3 Becomes PR-Shaped

By late September 2025, the branch vocabulary starts describing the pipeline rather than just the task:

- `feat/149-missing-docs-review`
- `feat/add-spec-149-governance-docs`
- `chore/ignore-integ-artifacts`
- `sync/master-commits-20250924-015945`

That lines up with the retained `.claude/agents4/` pack: `review`, `integration`, and `generative` are the operating phases. The repository stops looking like a stream of direct changes and starts looking like a staged PR conveyor.

## 3. January Names The Concerns

January 2026 is where the proto-specialist lanes become obvious. In that month, `bolt/`, `sentinel/`, and `palette/` show up as distinct branch families:

- `bolt/parser-alloc-opt-1662999377150898536`
- `sentinel/fix-command-injection-7117952845111742465`
- `palette/add-command-icons-8032879510220148309`

In the January slice I pulled, `bolt/` and `sentinel/` dominate the concern-lane set, with `palette/` as the smaller UX lane. That matches the `.jules` bridge: performance, security, and UX start to behave like named memory surfaces instead of one-off tasks.

## 4. Copilot And Release Waves

Late February and early March 2026 introduce a different branch vocabulary:

- `copilot/sub-pr-264`
- `wave/release-vscode-20260227`
- `release/public-crates-io-master`
- `release/v0.10.0`
- `release/turnkey-0.x.y`
- `rebase/local-history-20260227`
- `stabilize/nightly-ci-sweep-20260227`

These names read like campaigns. They are batch-oriented, release-oriented, and process-oriented. The branch is no longer just a task label; it is the lane the task belongs to.

## 5. The Current Swarm Uses Deterministic Worktrees

By March 11-19, 2026, `worktree-agent-*` becomes the key new naming pattern. In that window, the docs/article wave is obvious:

- `worktree-agent-af9eb608` -> `docs: add parser evolution deep dive for launch article series`
- `worktree-agent-a7d4ebda` -> `docs: add workspace architecture article`
- `worktree-agent-adda82ed` -> `docs: add LSP implementation story article`
- `worktree-agent-a4126d8a` -> `docs: add quality infrastructure deep dive article`
- `worktree-agent-ab8bc0d0` -> `docs: add agentic development history article`

The March slice I pulled still has many `codex/` branches, but `worktree-agent-*` is the new deterministic signal. It says: this is an isolated agent run, not just another ad hoc branch.

## 6. Titles Tell A Second Story

Branch names describe workflow. PR titles describe payload.

In the archive I pulled, the title families recur most often as `docs:`, `feat:`, `fix:`, `chore:`, `refactor:`, and `test:`. `docs:` is the biggest of those title families, which fits the late-stage article and reference-document waves that now sit beside product work.

The result is a clear progression:

1. direct implementation branches with imperative titles
2. Q3 PR-pipeline branches
3. January concern lanes
4. Copilot and release campaigns
5. deterministic `worktree-agent-*` swarm branches
6. docs/article waves as a first-class output

## Evidence Pointers

- [docs/project/AGENTIC_DEVELOPMENT.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_DEVELOPMENT.md)
- [docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_PR_ARCHAEOLOGY.md)
- [docs/articles/research/COPILOT_FLEET_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/COPILOT_FLEET_ARCHAEOLOGY.md)
- [docs/articles/research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ERA5_MIXED_TOOL_ARCHAEOLOGY.md)
- [docs/articles/research/SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- [docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)

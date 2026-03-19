# Era 5 Mixed Tool Archaeology
## March 11-19, 2026 Was A Mixed-Tool Window, Not A Single-Tool Sprint

Era 5 is easy to misread if you only look at the surface labels.

The git history shows a short Claude Code swarm period, but it sits inside a broader mixed-tool window where Codex CLI PR waves, `worktree-agent-*` branches, and the new `.claude` control plane all coexist. That makes this era qualitatively different from both the earlier Q3 swarm packs and the Copilot CLI firehose.

The right model is not "one tool won for a week."
The right model is "the repo had several parallel agent surfaces at once, and the control plane had become strong enough to absorb them."

---

## 1. March 11 Starts As A PR Collision, Not A Clean Tool Transition

The repository's own swarm-era history shows that March 11, 2026 opens with competing PRs for the same task:

- `#1244` `Add just doctor dev environment check and README quick-checks`
- `#1245` `Add just doctor developer environment check and onboarding script`
- `#1246` `chore(devex): add just devex quick environment check recipe`

That collision is important because it shows the period beginning with coordination pressure, not with a single authoritative workflow.

The historical note in `docs/project/AGENTIC_SWARM_ERA.md` explicitly describes this as three PRs appearing within minutes of each other, each solving the same problem differently. That is the shape of a mixed tool era: parallel agents converge on similar work, the human selects the best path, and the repo keeps moving.

---

## 2. Claude Code Became The Control Plane, Not The Whole Story

The current `.claude` runtime surfaces explain why Era 5 should not be treated as a simple "Claude era":

- `.claude/agents/` is the archived roster surface
- `.claude/commands/` holds slash entrypoints
- `.claude/skills/` holds reusable procedures
- `.claude/hooks/` enforces behavior deterministically
- `.claude/swarm-state/` stores durable queue, dedup, pitfalls, and findings state

The March 15 commit `9cc2d3b9a` (`feat(swarm): continuous swarm infrastructure with agent teams (#1553)`) turns that control plane on. The next few days then split the model further:

- `bf23f7904` restructures the team from 12 to 5 coordinators
- `1fd8f7e36` adds invocation-control frontmatter to skills
- `e4a089ef4` adds hooks
- `d9aab31bc` and `37ddcf56d` turn `swarm-state` into a schema-backed ledger

That means Claude Code in Era 5 is not just "the tool that wrote code." It is the orchestration layer that made the mixed-tool environment tractable.

---

## 3. Codex CLI Was Still Very Much In The Room

Git history on March 18-19 shows a large stream of merges from `codex/*` branches:

- `Merge pull request #2006 from EffortlessMetrics/codex/code-lens-docs-align`
- `Merge pull request #1890 from EffortlessMetrics/codex/improve-the-parser-p0uj0x`
- `Merge pull request #1862 from EffortlessMetrics/codex/document-apparent-adrs-in-codebase-urhje8`
- `Merge pull request #1774 from EffortlessMetrics/codex/improve-fuzzing-coverage`
- `Merge pull request #1811 from EffortlessMetrics/codex/split-srp-microcrates-and-prepare-pr-98hudp`

That is not the signature of a single-tool era. It is the signature of concurrent agent populations:

- Claude Code bursts are handling orchestration, cleanup, and targeted swarm runs
- Codex CLI is still generating a large PR stream
- the human remains the merge and triage bottleneck

The repo history therefore supports a mixed-tool conclusion rather than a monoculture conclusion.

---

## 4. `worktree-agent-*` Shows The Swarm Had Matured Into A Branching Discipline

The March 16 and March 19 merge history includes branches named like:

- `worktree-agent-ac95189b`
- `worktree-agent-a7e6f97b`
- `worktree-agent-a90d7ded`
- `worktree-agent-a638b42e`
- `worktree-agent-a5823b8f`
- `worktree-agent-ab615443`

That naming pattern matters because it is not a generic branch convention. It is the repo encoding isolation into the branch name itself.

By Era 5, the control plane has a stable write boundary:

- a worktree per slice
- a named worker per task
- a control-plane surface for coordination
- a branch name that carries the agent identity

This is why the era reads as a system, not a single series of prompts.

---

## 5. Why This Is Not A Commits-Per-Day Race

The period from March 11 to March 19 includes:

- direct PR collisions on March 11
- the March 15 swarm-infrastructure turn-on
- the March 16 extraction of commands, skills, and hooks
- the March 17 findings ledger and roster contract
- the March 18-19 merge stream from both `worktree-agent-*` and `codex/*` branches

That timeline is too mixed to summarize honestly as "one tool drove the commits/day curve."

The better reading is:

- short Claude Code swarm bursts handled coordination and cleanup
- Codex CLI continued producing PR waves
- the control plane got strong enough to absorb both
- the human still arbitrated merge order and repo-level judgment

So the era is best described as a mixed-tool operating window with short swarm runs, not as a sustained single-tool throughput race.

---

## 6. Evidence Pointers

- [docs/project/AGENTIC_SWARM_ERA.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/AGENTIC_SWARM_ERA.md)
- [docs/articles/research/BLOG_MATERIAL_INDEX.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/BLOG_MATERIAL_INDEX.md)
- [docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/CONTROL_PLANE_ARCHAEOLOGY.md)
- [docs/articles/research/SWARM_SURFACE_EVOLUTION.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/SWARM_SURFACE_EVOLUTION.md)
- `.claude/README.md`

Key commits:

- `9cc2d3b9a` - continuous swarm infrastructure with agent teams (`2026-03-15`)
- `bf23f7904` - restructure team from 12 to 5 coordinators (`2026-03-16`)
- `1fd8f7e36` - invocation-control frontmatter for skills (`2026-03-16`)
- `e4a089ef4` - add hooks (`2026-03-16`)
- `d9aab31bc` - durable findings schema and ledger (`2026-03-17`)
- `37ddcf56d` - empty-ledger validation hardening (`2026-03-17`)
- `cb4251735` - archived agent iterations explicitly retained (`2026-03-19`)
- `99d2b17f0` - revert of attempted lineage cleanup (`2026-03-19`)

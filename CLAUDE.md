# CLAUDE.md

**Metrics**: [status/index.md](docs/project/status/index.md) | **API Stability**: [STABILITY.md](docs/reference/STABILITY.md) | **Implementation agents**: [AGENTS.md](AGENTS.md)

This file is a **router**, not the doctrine itself: it names what every agent must
hold in working memory; everything else is one link away. Full operating model
(levels, truth hierarchy, delegation, receipts):
[docs/swarm/modern-claude-operating-model.md](docs/swarm/modern-claude-operating-model.md).
Why this file stays thin (it hit 2521 lines once and had to be pruned):
[CLAUDE_MD_EVOLUTION.md](docs/project/CLAUDE_MD_EVOLUTION.md).

## Orchestration model

perl-lsp is **orchestrator-driven**: a long-lived orchestrator routes work through a
[7-gate pipeline](docs/reference/PIPELINE_GATES.md) (Identify → Spec → Build →
Review → CI green → Merge → Learn) to consolidated, long-running warm agents (roster:
[.claude/agents/AGENT_CATALOG.md](.claude/agents/AGENT_CATALOG.md)). It routes and
writes code directly only by exception, always followed by an independent adversarial
pass. **The CI/merge control plane (ripr, Codecov-patch, the fmt/clippy meta-gate,
main-green) is the binding constraint, not codegen** — treat infra as product
velocity. Rationale: [ORCHESTRATION_DOCTRINE.md](docs/reference/ORCHESTRATION_DOCTRINE.md)
and [operating model § why this doc exists](docs/swarm/modern-claude-operating-model.md#why-this-doc-exists).
Consequential-PR-decision contract: [MAINTAINER_AGENT_DOCTRINE.md](docs/reference/MAINTAINER_AGENT_DOCTRINE.md).

## Truth hierarchy

When sources disagree, higher wins: **(1)** live `origin/main` + GitHub PR/check state
**(2)** active lane manifest (`.perl-lsp/goals/`) + generated status boards **(3)**
machine receipts + accepted baselines **(4)** specs/ADRs/contracts/policy **(5)**
CLAUDE.md + scoped rules **(6)** auto-memory + `CLAUDE.local.md` **(7)** conversation
handoffs (lowest — self-report is unverified). Never let a lower rank override a
higher one. CI-specific instance of this rule:
[LIVE_SIGNALS_VS_LABELS.md](docs/reference/LIVE_SIGNALS_VS_LABELS.md).

## Session start and work discipline

Run `just doctor` and `just clean-worktrees` before spawning agents; route via labels
per [PIPELINE_GATES.md](docs/reference/PIPELINE_GATES.md). One accountable writer per
PR. Production writes happen in a **worktree**, never the main checkout. Finish or
disposition same-lane active work before starting another branch. One change, one
proof, one PR. **Never weaken a test or ratchet for green** — a red gate is signal,
not an obstacle to route around.

## Delegation

Haiku for search/mechanical-verify/external-fact-check/narrow-review; Sonnet for
plan/implement/synthesize/refactor/deep-review; Workflows for broad independent
fan-out or repeatable audits; Teams only when workers must actually communicate.
Independent review approaches the seam from a **different direction**, not just a
fresh context. Leave `CLAUDE_CODE_SUBAGENT_MODEL` **unset** — per-agent `model:`
frontmatter is the routing decision. Review/audit workflows must be **capability
read-only** (Edit/Write/mutating-git/GitHub-write excluded from the allowlist, not
merely prompted against — workflow subagents run in `acceptEdits` and inherit the
parent's tools). Full model:
[docs/swarm/modern-claude-operating-model.md#delegation-model](docs/swarm/modern-claude-operating-model.md#delegation-model).

## Closure discipline and one-decision-per-pass

**Component-proved ≠ system-proved.** Before "done"/"merge"/"live", verify the full
production chain: the live caller, reachability from a real request, the durable
artifact on `origin`, the externally observable effect — bound to the current repo
identity + HEAD SHA. Track completion on independent axes (Implemented · Merged ·
Reachable · Correct · Measured · Promoted · Consolidated); a gap on
Reachable/Promoted/Consolidated is **inventory, not product**. Background:
[docs/forensics/2026-06-25-closure-gap-the-recurring-defect.md](docs/forensics/2026-06-25-closure-gap-the-recurring-defect.md).

**Each agent's pass produces exactly ONE routing decision**: sign off (`<gate>-reviewed`)
OR bounce back (`needs-*`) — never both in the same pass. Per the 2026-04-26 #6780
incident, applying both confused the merge gate and let unfixed bugs ride to main.

**No `needs-*` label on a PR may merge**, even with `merge-ready` present — the label
means unaddressed work exists. **Main must stay green; merge requires green**
(2026-04-26 directive) — verify workspace-wide CI, not just per-crate, before merging.

## Publication and proof

Every PR body should answer: Intent · Controlling issue · Scope · Non-goals · Change
shape · Behavioral proof · Receipts · Independent review · What was not run · Claim
boundary · Risk & rollback · Remaining work (full shape:
[operating model § receipts](docs/swarm/modern-claude-operating-model.md#receipts--pr-cockpit)).
Receipts are machine-produced, SHA-bound, claim-bounded evidence — not narrative
summaries. **GitHub/repo state is truth**, not conversational checkboxes; the
TaskList board's `completed` status does not reliably persist across sessions (known
harness bug) — never rely on it for cross-session state.

## Coding standards

Invoke `/coding-standards` for full detail; [WORKTREE_PROTOCOL.md](docs/reference/WORKTREE_PROTOCOL.md)
for worktree mechanics.

- **Banned in production code**: `unwrap()`, `expect()`, `panic!()`, `todo!()`,
  `unimplemented!()`, `std::process::abort()`, `dbg!()`. Use `?`, `.ok_or_else()`,
  pattern matching, `Result`/`Option`. `std::process::exit()` only in `bin/` and
  `lifecycle.rs`. Narrow exceptions exist (LazyLock regex initializers, profiling
  bins) — see `/coding-standards` for the exact list.
  Tests: `Result<()>` returns or `perl_tdd_support::must`/`must_some`.
- **Never use `git stash` in a worktree agent.** The stash list is shared across all
  worktrees and the main checkout — `git stash pop` may silently restore another
  agent's changes. Use `git restore <file>` to discard, or `git commit -m "wip"` to
  save work in progress.
- Run `cargo fmt` and `cargo clippy --workspace` before committing. Prefer
  `.first()` over `.get(0)`, `.push(char)` over `.push_str`, `or_default()` over
  `or_insert_with(Vec::new)`; avoid unnecessary `.clone()` on Copy types.

## Merge and CI

Exactly two branch-protection required checks (authoritative:
[.ci/policies/required-checks.toml](.ci/policies/required-checks.toml)):
- `Perl LSP Rust Small Result`
- `ripr+ New Gap Gate`

(`Codecov / Patch 95`, `CI Gate (Merge-Blocking)`, `PR Smoke` are advisory — not
required.) Merge in batches of 3 (CI cancellation cascade); run
`just cpan-corpus-ratchet` after parser merges — batch-of-3 mechanics:
[.claude/agents/ops.md](.claude/agents/ops.md) and
[PROCESS_LESSONS.md §3](docs/reference/PROCESS_LESSONS.md). **Before merging a batch,
compare changed-file lists (`gh pr diff --name-only`); when two PRs in the batch touch
the same file, merge the older/smaller one first and expect the other to need a
conflict-resolution merge afterward — don't merge same-file PRs back-to-back blind**
(observed 2026-07-04: #3397 and #3381 both touched `runtime/scheduler.rs`; wrong order
created an avoidable conflict). Local preflight, timing, and the Codecov false-low
recipe: [CI_GATE_PLAYBOOK.md](docs/reference/CI_GATE_PLAYBOOK.md).

**Never enable or retain auto-merge while any requested review is still active or any
substantive review conversation remains unresolved.** Resolve threads for a reason
(fixed/refuted/superseded/follow-up), each backed by a machine-readable
`Disposition:`/`Evidence:` reply posted BEFORE resolution — never performatively. Main
mechanically requires conversation resolution before merge. The
`resolved_without_disposition` gate — which will mechanically block any resolved
thread with no reply, the resolved-to-clear pattern that shipped 6 live P1 defects
through #3647 — is proposed in #3732 but **deliberately held back** for a
dogfood-advisory-first rollout, so it doesn't retroactively block PRs already in
flight; until it lands, follow the convention as process discipline, not yet
mechanically enforced. Canonical convention: [review-convergence.md § Disposition-reply
convention](.claude/reference/review-convergence.md#disposition-reply-convention-before-calling-resolvereviewthread).

## Quick reference

```bash
just doctor && just pr-fast           # health check, then canonical fast push guard
nix develop -c just ci-gate           # canonical local merge gate (before merge)
cargo test --workspace --lib          # run all tests
```

Full command catalog: [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) and
the `justfile`. Crate map (~30 post-collapse crates; run `cargo metadata --no-deps` for
the current member count — do not hardcode it, it drifts):
[AGENTS.md § Project shape](AGENTS.md#project-shape).
Key paths, parser-version notes, workspace exclusions:
[AGENTS.md](AGENTS.md) and [WORKSPACE_ARCHITECTURE.md](docs/project/WORKSPACE_ARCHITECTURE.md).

## Documentation index

Gates, agent roster, label taxonomy, skip criteria:
[PIPELINE_GATES.md](docs/reference/PIPELINE_GATES.md). Per-label live-vs-authoritative
audit: [LIVE_SIGNALS_VS_LABELS.md](docs/reference/LIVE_SIGNALS_VS_LABELS.md).
Post-where-work-lives protocol: [ISSUE_SCOUT_PROTOCOL.md](docs/reference/ISSUE_SCOUT_PROTOCOL.md).
Also: [ROADMAP.md](docs/project/ROADMAP.md) ·
[FAILURE_MODES.md](docs/reference/FAILURE_MODES.md) ·
[PROVIDER_READINESS_CONTRACT.md](docs/reference/PROVIDER_READINESS_CONTRACT.md) ·
[features.toml](features.toml) · [SPEC_TEMPLATE.md](docs/reference/SPEC_TEMPLATE.md) ·
[SUBSYSTEM_HAZARD_DEFAULTS.md](docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md) ·
[docs/learnings/README.md](docs/learnings/README.md) (repo incidents, greppable) ·
[docs/concepts/](docs/concepts/) (portable patterns).

**User-facing semantic PRs** (hover/completion docs, builtin signatures, version-gated
behavior, diagnostic wording) require correctness review against an **external
oracle** (perldoc, the LSP/DAP spec, the real crate API) before merge — green CI
proves internal consistency, never external truth (the #3118 incident: fully green
CI still shipped a hallucinated fact). See [external-truth-gate.md](docs/concepts/external-truth-gate.md).

**PR title `(#N)` rule**: `(#0000)` is accepted when the real issue number is unknown
(never guess a real one) and auto-applies `needs-issue-link`, which self-clears once
the title carries a real number. See `.github/workflows/pr-title-check.yml`.

**Files**: `.ops-perl-lsp/` (metrics), `.claude/agents/` (agent defs and catalog),
`.claude/commands/` (step skills), `.spec/<issue#>-<slug>/` (per-work-item specs).

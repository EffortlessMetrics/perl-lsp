# AGENTS.md — Implementation Agent Operating Manual

You are an **implementation agent** (Codex, Claude Code, Jules, or similar). Your job is to make a
scoped change, test it, and open a PR. You are not the orchestrator. You will not be
routing work or reading CI pipelines — just implement the thing you were asked to implement.

The orchestrator reads `CLAUDE.md`. This file is for you.

---

## Context you can read

| Resource | Purpose |
|----------|---------|
| `README.md` | What this project is |
| `CLAUDE.md` | Orchestrator pipeline model (routing, labels, merge rules) |
| `docs/project/ROADMAP.md` | Active waves and priorities |
| `docs/project/FRICTION_LOG.md` | Platform quirks and known workarounds |
| `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md` | The orchestration pattern used here |
| `docs/articles/ORCHESTRATION_COUNTERINTUITIONS.md` | Lessons where the obvious rule was wrong |
| `docs/agents/ORCHESTRATION_ROLES.md` | Role taxonomy: model tier, constraints, output schema per role |
| `docs/agents/CLOSE_PROOF_POLICY.md` | Mandatory proof artifacts before closing any issue as superseded/landed |
| `docs/agents/SCOUT_PROMPTS.md` | Ready-to-paste haiku scout prompt templates (7 templates: classify-PR, verify-duplicate, verify-reachability, read-CI-failure, audit-docs-claim, classify-issue, classify-release-blocker) |
| `docs/agents/BUILDER_BRIEF_TEMPLATE.md` | Sonnet builder brief template with all required fields + known-gotchas appendix |
| `docs/agents/WORKFLOW_TEMPLATES.md` | Outline-level templates for the six queue-processing workflows (pr-classifier, issue-triage, source-swarm-reconciliation, release-readiness, ci-failure-cluster, ub-review-calibration) |
| `docs/agents/EVIDENCE_STANDARD.md` | Evidence ban-list and required-artifact table by claim type |
| `docs/agents/CLEANUP_CHECKLIST.md` | Post-wave cleanup checklist: worktrees, branches, target dirs, stale processes, storage doctor |
| `docs/agents/pr-ledger.schema.json` | JSON Schema for PR reconciliation ledger rows (used by pr-classifier and source-swarm-reconciliation workflows) |

**Before starting:** check the latest upstream commits so you do not re-implement
already-merged work.

```bash
# Preferred (when origin/master exists)
git log origin/master --oneline -20

# Fallback for local-only clones/worktrees
git log --oneline -20
```

The orchestrator frequently merges fixes between sessions — your task may already be done.

**Before stating facts** about workspace counts, release numbers, or metrics, verify
against the truth sources below — do not hardcode them:

- [`Cargo.toml`](Cargo.toml) — workspace members, package version
- [`docs/project/CURRENT_STATUS.md`](docs/project/CURRENT_STATUS.md) — evidence-backed metrics
- [`docs/project/ROADMAP.md`](docs/project/ROADMAP.md) — canonical roadmap
- [`features.toml`](features.toml) — LSP capability catalog

---

## Project shape

| Path | Purpose |
|------|---------|
| `crates/perl-lsp-rs/` | LSP binary and server host |
| `crates/perl-dap/` | Debug Adapter Protocol server |
| `crates/perl-parser/` | Native recursive-descent parser |
| `crates/perl-lexer/` | Context-aware tokenizer |
| `crates/perl-parser-core/` | Shared parser infrastructure |
| `crates/perl-semantic-analyzer/` | Semantic analysis and resolution |
| `crates/perl-workspace-index/` | Cross-file indexing and lookup |

Families: `perl-module-*` (module resolution), `perl-lsp-*` (LSP providers),
`perl-lsp-feature-*` (feature governance), `perl-dap-*` (DAP), `perl-workspace-*`
(workspace discovery).

---

## Scoping your PR

- Touch **one concern**. One fix, one feature, one refactor.
- Do NOT bundle unrelated cleanup — "while I'm here" creates review burden and scope drift.
- Do NOT drop, `#[ignore]`, or comment out existing tests — that is named debt.
- Do NOT rewrite files you did not need to touch.
- If your branch contains tool metadata (`.hermes/conveyor/work-<id>/`), that is fine
  to include — but do NOT mix multiple work IDs in one PR.
- Do NOT commit top-level `adr.md`, `specs.md`, or `task_list.md` — those belong in
  `.hermes/conveyor/work-<id>/`.

---

## Commit and PR conventions

**Commit:** Single focused commit. Squash locally before pushing.

**Title format:** `type(scope): description (#NNNN)`

- If you cannot read the issue tracker, use `(#0000)` — the orchestrator will retitle
  before merge. The CI `validate-title` check enforces the `(#NNNN)` suffix.
- Valid types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`.

**PR body template (keep it short):**

```
Problem: <one sentence>
Fix: <one sentence>
Verification: `cargo test -p <crate>` passes / `just pr-fast` passes
```

---

## Code quality bar

These rules apply to **all** production code and tests. CI and reviewers enforce them.

**Banned — use `?`, pattern matching, or `Result`/`Option` instead:**

```
panic!()    unwrap()    expect()    todo!()    unimplemented!()
dbg!()      println!()  eprintln!()  (in library code — use `tracing`)
```

**Tests** must return `Result<()>` or use `perl_tdd_support::must` / `must_some`.
No bare `unwrap()` in tests.

**Regex:** declare as `static LazyLock<Regex>` — never call `Regex::new()` per invocation.

**Public API on facade crates:** add `#[non_exhaustive]` to enums and structs.

**Async:** do not hold a lock across `.await` — clippy `await_holding_lock` is denied.

**Every `#[allow(...)]`** must have a justification comment on the same or preceding line.

**Prefer:**
- `.first()` over `.get(0)`
- `.push(ch)` over `.push_str("x")` for single chars
- `.or_default()` over `.or_insert_with(Vec::new)`
- Avoid unnecessary `.clone()` on `Copy` types

---


## Build storage discipline

This repo is often used with many disposable worktrees and agent sessions.
Do not create large build output inside the worktree.

Preferred commands:

```bash
just agent-check
just agent-test
just agent-clippy
just agent-pr-fast
```

For direct Cargo, use:

```bash
./scripts/cargo-safe check -p <crate> --all-targets --profile agent --locked
./scripts/cargo-safe test -p <crate> --profile agent --locked
./scripts/cargo-safe clippy -p <crate> --profile agent --locked -- -D warnings -A missing_docs
```

Avoid:

```bash
cargo test --workspace
cargo check --workspace --all-targets
cargo clean
rm -rf target
```

unless the orchestrator explicitly assigned a build lane.

Storage invariant:

```bash
./scripts/storage-doctor
```

must not show large repo-local `target/` directories.

---

## Verification before pushing

```bash
./scripts/cargo-safe test -p <crate> --profile agent --locked
./scripts/cargo-safe check --all-targets -p <crate> --profile agent --locked
./scripts/cargo-safe xtask fmt
./scripts/cargo-safe clippy -p <crate> --profile agent --locked -- -D warnings -A missing_docs
just agent-pr-fast
```

For the canonical local merge gate (before a reviewer merges):

```bash
nix develop -c just ci-gate
```

---

## Platform awareness

- CI runs on Linux. Code must be cross-platform — Windows CRLF and Unix LF both occur
  in the corpus.
- You cannot read GitHub issues directly. Placeholder issue refs (`#0000`) are OK;
  the orchestrator retitles before merge.
- If the pre-push hook fails on pre-existing fmt drift in unrelated files, you may use
  `--no-verify` but must note it explicitly in the PR body.
- `git stash` is **shared across all worktrees** — never use it. Use `git restore <file>`
  to discard changes, or `git commit -m "wip"` to save in-progress work.
- Pre-push hook may hit a file-lock race on Windows; API-push workaround is acceptable.
  See `docs/project/FRICTION_LOG.md` for details.
- If you are running via Codex CLI in a non-interactive environment, do not pause for
  confirmation prompts. Run required checks/commands directly and report outcomes.

---

## Anti-patterns (explicit DON'T list)

| Pattern | Why |
|---------|-----|
| Multiple `.hermes/conveyor/work-<id>/` dirs in one PR | Cross-work-id contamination |
| Committing `adr.md` / `specs.md` at repo root | Belongs in conveyor dir |
| Rewriting files you did not need to touch | Creates spurious review surface |
| Dropping or `#[ignore]`-ing tests | Debt with a name |
| Guessing issue numbers | Placeholder `#0000` is fine; wrong ref misleads reviewers |
| Bundling unrelated changes in one PR | Scope drift kills reviewability |
| Using `git stash` | Shared across worktrees — use `git restore` or `git commit -m "wip"` |
| Hardcoding metrics in new docs | Metrics drift; link to truth sources instead |
| **Closing another lane's PR** | See "Lane scope" below — out-of-scope action, always |

---

## Lane scope: stay in your lane

This repo coordinates several implementation agents running in parallel
**lanes**. A lane owns a scoped queue of issues, PRs, and branches,
identified by a `lane: <N>` label on those PRs. Your lane is whatever
the orchestrator (or the maintainer) routed to you.

**You may only act on PRs that belong to your lane.** "Act on" means:

- close, reopen, force-push, merge, request changes;
- apply `needs-*` routing labels;
- open competing PRs for the same change.

You may always **read** any PR, and you may **comment** on any PR with
on-point feedback. Anything that *changes the state* of a PR — most of
all closing it — is scoped to that PR's lane.

**Your lane's burn-down does not include closing other lanes' PRs.**

Cleanup, control-plane hygiene, queue management, "we want a quiet
queue" — none of these are justifications to close another lane's
in-flight work. Cleanup means finishing **your own** lane. If another
lane's PR is creating risk for yours, the response is:

1. Comment on the PR with the specific risk you see.
2. Tag the maintainer for arbitration.
3. **Wait.** The maintainer arbitrates lane priority.

Closing another lane's PR is **never** the next step. A maintainer
override ("explicitly resumed", "this is not your lane", "direct
maintainer permission") immediately supersedes any standing lane posture
you were operating under — back out of the action you were about to
take.

For the full lane model (how lanes are identified, override semantics,
cross-lane communication), see
[`docs/reference/LANE_BOUNDARIES.md`](docs/reference/LANE_BOUNDARIES.md).

---

## Documentation discipline

- `docs/project/CURRENT_STATUS.md` is the evidence document. Do not duplicate its tables.
- `docs/project/ROADMAP.md` is the planning document. Keep "shipped" and "targeted" separate.
- Prefer links to canonical docs over copying the same table into multiple files.
- After adding tests: no manual status update needed — `docs/project/status/*.md` files
  are auto-regenerated post-merge via `just status-update`.

---

## Further reading

- `CLAUDE.md` — full orchestrator model, pipeline stages, label semantics
- `CONTRIBUTING.md` — human contributor workflow
- `docs/project/FRICTION_LOG.md` — platform quirks and workarounds
- `docs/reference/LANE_BOUNDARIES.md` — lane model: scope, overrides, non-overlap rule
- `docs/reference/COMMANDS_REFERENCE.md` — all `just` and `cargo xtask` commands
- `docs/reference/LSP_IMPLEMENTATION_GUIDE.md` — LSP provider patterns
- `docs/reference/CRATE_ARCHITECTURE_GUIDE.md` — microcrate layering rules
- `docs/articles/ORCHESTRATION_COUNTERINTUITIONS.md` — where the obvious rule was wrong

# 2026-05-11 — Autonomous Session Learnings

**Session**: Autonomous-mode loop, ~9 hours, ~15 PRs  
**Workstreams**: @INC strictness, Rust 1.95 rollout, CI economics, non-rust file policy, Codecov rollout  
**Tracking issue**: #8548

---

Each entry has: (1) one-line takeaway, (2) incident, (3) what to do differently, (4) where durably encoded.

---

## 1. Stale-checkout is silent destruction

**Takeaway**: Before filing any claim about code state, run `git fetch origin master` and verify the specific file from `origin/master`. There is no in-loop feedback when the checkout is stale.

**Incident**: Session opened with `Read` on `crates/perl-lsp-rs/src/runtime/language/completion.rs` and concluded `perl5lib_paths_for_completion` did not exist. The local checkout was 5 commits behind `origin/master`. The function existed. This misfiled 8 @INC rollout issues. ChatGPT-Pro architectural review caught it; the user manually corrected the issue bodies. Cost: ~30 minutes of issue rewrites and scope corrections.

**Next time**:
- Before any `Read`/`Grep`/`Glob` used to make a *forward claim* (filing an issue, briefing a builder, writing a doc citing "X currently does Y"), run: `git fetch origin master && git log --oneline HEAD..origin/master`. If behind, read from `origin/master` explicitly or use a fresh worktree pinned to it.
- Classifying question: "Am I about to claim something about code state that another agent or human will act on?" If yes, fetch first.
- Routine edits inside a known feature branch do not require a fetch.

**Durably encoded**: `~/.claude/projects/H--Code-Rust2-perl-lsp/memory/feedback_stale_checkout.md`. Pre-tool hook proposal tracked as tooling-debt (separate issue).

---

## 2. Sonnet should not do discovery

**Takeaway**: Haiku scouts the implementation map and files it as a GitHub issue; sonnet builder implements against that spec. Skipping the haiku scout makes the sonnet builder spend its entire run on preflight.

**Incident**: A sonnet builder spawned for #8537 with an incomplete brief spent ~28 minutes on preflight (reading files, tracing call sites, mapping the workspace) and produced zero code. After a SendMessage with explicit `file:line` references it became productive immediately. The scout work that should have happened before the builder was instead happening inside the builder's run.

**Next time**:
1. Run a haiku scout (skill: `scout-issue`, or a generic `Explore` agent) that produces an implementation map: exact file paths, function signatures to touch, helpers to reuse, acceptance tests.
2. Scout files that map as a GitHub issue via REST (`gh api repos/.../issues --method POST`; not `gh issue create` — GraphQL rate-limited).
3. Builder brief references the issue number and includes "Closes #N" in the PR.
4. Tell the builder "preflight has been done; proceed to implementation" to skip redundant filesystem checks.
- Exception: one-shot tiny edits where `file:line` is already in working memory (README badge swap, 1-line clippy-allow removal) do not need a separate scout pass.

**Durably encoded**: `~/.claude/projects/H--Code-Rust2-perl-lsp/memory/feedback_builder_pipeline.md`. Tooling-debt: #8542.

---

## 3. Coworker agents are signal, not noise

**Takeaway**: codex, factory-droid, and similar bots ship real fixes in adjacent territory; read their output before starting work that overlaps.

**Incident**:
- Codex shipped #8519 (CI aggregator lane-whitelist) within an hour of #8510. Complementary work; would have been a conflict if not noticed.
- Codex shipped #8523 (`xtask/src/tasks/metrics/parser_stats.rs` `sort_by_key` cleanup) catching a site missed by `--lib`-only clippy in #8511. Redundant work narrowly avoided.
- Factory-droid Phase-2 validator on #8512 flagged two real P1 bugs (incomplete JSON validation; no test for the published docs file). Those fixes would not have shipped without the bot comment.

**Next time**:
- When triaging open PRs, scan non-mine PRs for adjacent territory (same crate, same workstream) before starting: `gh api repos/.../pulls?state=open`.
- Read the factory-droid comment on every PR you spawn; it's the most reliably useful bot review in this repo.
- If codex already opened a PR matching what you're about to start, drop your branch and merge theirs.
- Bot branch pattern: `claude/`, `codex/`, `dependabot/`. Bot user pattern: `*[bot]`.

**Durably encoded**: `~/.claude/projects/H--Code-Rust2-perl-lsp/memory/feedback_coworker_agents.md`.

---

## 4. PR title `#NNNN` is a hard contract

**Takeaway**: The `validate-title` workflow fails any PR whose title does not contain `(#NNNN)`. This is not optional; it is enforced by CI.

**Incident**: Multiple PRs in the session triggered `validate-title` failures because the issue number was missing from the title. Each failure cost ~5 minutes of CI retry time plus a force-push to fix. Across 15 PRs, this accumulated.

**Next time**:
- Format: `<type>(<scope>): <description> (#NNNN)` — the issue number in parentheses is mandatory.
- File the tracking issue *before* writing the commit message so the number is available.
- The haiku-scout-files-issue rule (item 2 above) structurally enforces this: the issue exists before the builder starts.
- Pre-push hook proposal: lint PR title before push. Tracked as tooling-debt.

**Durably encoded**: CLAUDE.md pipeline contract; pre-push hook tooling-debt tracked separately.

---

## 5. CI cost dominates diff cost under agent merge velocity

**Takeaway**: Under autonomous-agent merge velocity, batching by workstream is cheaper than micro-PRs even when each micro-PR's diff is trivial.

**Incident**: Early in the session, several micro-PRs were filed for adjacent clippy-allow removals in the same crate. Each PR consumed ~6–10 minutes of CI runtime regardless of diff size. At ~15 merges/session, CI runtime (not diff complexity) became the binding constraint. Batching the clippy-allow removals by crate into single PRs would have halved CI runtime for the same net change.

**Next time**:
- When filing multiple PRs in the same crate/workstream, check: can these be one PR without violating "do not combine" rules (e.g., MSRV bump + lint activation must stay separate)?
- Each PR is ~6–10 min of CI regardless of diff size. Three 1-line PRs in the same crate = 18–30 min of CI. One 3-line PR = 6–10 min.
- "Do not combine" overrides batching: MSRV bump, lint activation, no-panic baseline, release bump must each be separate PRs.

**Durably encoded**: `docs/development/RUST_1_95_ROLLOUT.md` "do not combine" section; `docs/ci/codecov-rollout.md` PR ladder pattern.

---

## 6. Hold the user's last-stated workstream — do not drift

**Takeaway**: When no new user signal arrives, continue the last-stated workstream. Do not drift to adjacent work that another bot is already handling.

**Incident**: The user set the workstream as "@INC strictness and UX completion features." During a gap in user messages, the orchestrator drifted to clippy-allow removal cleanup — the same rail codex was already working. The @INC follow-ups sat queued while codex's pattern got replicated. The user's corrective message ("Or, actually, lets improve around @inc instead") was an 8-word pivot that cost the user attention to make because the orchestrator had defected from the brief.

**Next time**:
- Track the active workstream label explicitly. Before starting any new sub-task, ask: "Is this in the last-stated workstream, or am I drifting?"
- Adjacent-bot work (codex cleaning up clippy-allows) is a signal to *not* duplicate it, not to join it.
- If the last-stated workstream is exhausted and no new signal has arrived, surface "workstream complete, waiting for direction" rather than self-assigning from adjacent queues.
- Terse redirects are high-precision instructions; re-read before acting. "Lets improve around @inc" vs "lets improve @inc" differ by one word and change scope.

**Durably encoded**: `~/.claude/projects/H--Code-Rust2-perl-lsp/memory/feedback_user_attention_cost.md`.

---

## 7. `policy/` directory is queryable control-plane state

**Takeaway**: The `policy/` TOML files are structured, machine-readable decisions — treat them as queryable state, not as documentation to read once and ignore.

**Context**: The repo has 10+ TOML files in `policy/` (ci-budget, ci-lanes, clippy-debt, clippy-lints, non-rust-allowlist, etc.). Each file encodes a decision that affects CI, lint gates, or merge behavior. Agents that read `Cargo.toml` or CLAUDE.md but skip `policy/` will miss hard constraints.

**Next time**:
- Before writing any CI configuration, lint gate, or non-rust file handler, `ls policy/` and read the relevant file.
- Before claiming "CI allows X" or "lint gate is set to Y", verify against `policy/ci-budget.toml` or `policy/clippy-lints.toml` respectively.
- `policy/non-rust-allowlist.toml` is the authoritative list for what non-Rust files are permitted; the non-rust inventory in #8512 was derived from it.
- When adding a new type of constraint (a new lint tier, a new CI lane), file it as a new `policy/` TOML entry, not as a comment in `Cargo.toml`.

**Durably encoded**: No dedicated memory file yet. This doc is the first encoding.

---

## 8. Repo CI design: green deep-scoped CI plus automerge, not trust without checks

**Takeaway**: The repo merges without required human reviews because CI is tightly scoped per-crate and actually decisive — not because it's trust-based. Automerge triggers on green CI, not on absence of objection.

**Context**: `mergeable_state: unstable` (GitHub's "checks not yet completed") is mergeable here once CI passes. No required reviewer approval. This is intentional: each PR's CI scope is narrow enough that green = verified. The design is fast-decisive checks, not no-checks.

**Next time**:
- Do not frame the merge model as "trust-based" or "no oversight." It's "decisive scoped CI."
- Before merging, the green-ci gate verifies CI is green on the *current HEAD SHA* — a stale-green from a prior push is not sufficient.
- `mergeable_state: unstable` only means "CI in progress." If CI passes, it becomes mergeable.
- The workspace-wide `xtask fmt` and `clippy` cascade is the reason per-PR CI green is necessary but not always sufficient — a single-crate green PR can introduce a workspace-wide fmt drift. Verify workspace-wide before merging anything that touches `xtask/` or shared derives.

**Durably encoded**: CLAUDE.md ("Master must stay green; merge requires green" directive, 2026-04-26).

---

## 9. Layered docs require reciprocal cross-link banners

**Takeaway**: When a second doc refines or supersedes a first, add a reciprocal cross-link banner to both. Do not delete the older doc; the history layer is the value.

**Incident**: After `docs/ci/codecov-rollout.md` was created as a post-landing improvement doc, the orchestrator considered deleting `docs/development/RUST_1_95_ROLLOUT.md` as a "duplicate." The user corrected: "We can have layered documentation of this." The two docs serve different functions (initial rollout ladder vs. post-landing quality improvement). Deleting either would have lost the scoped framing the other doc provides.

**Next time**:
- When creating a doc that refines or extends an existing one, add a banner to both: `> See also: [<other-doc>](<path>) — <one-line relationship>`.
- Decide relationship explicitly: "supersedes" (old is now wrong), "refines" (old is still valid context), "extends" (new adds depth). Don't delete unless the old doc is actively misleading.
- The test: can an agent landing on only the old doc still execute correctly? If yes, keep both and link them.

**Durably encoded**: `~/.claude/projects/H--Code-Rust2-perl-lsp/memory/feedback_docs_for_agents.md` (partial). This doc encodes the anti-delete rule specifically.

---

## 10. Claim boundaries belong in every quality-rollout doc

**Takeaway**: Every quality-rollout doc must explicitly declare what it proves and what it does not prove. "100% coverage" without a claim boundary is misleading.

**Incident**: The Codecov rollout doc initially stated coverage goals without claim boundaries. The plan-reviewer and user pushed back: a coverage percentage proves no-regression against baseline; it does not prove correctness. The distinction matters because agents reading the doc to evaluate "is quality sufficient?" will over-interpret a bare percentage.

**Next time**:
- Every quality metric in a rollout doc gets a "claim boundary" section:
  ```
  Proves: no-regression against baseline (baseline-ratchet enforces this)
  Does not prove: correctness, absence of untested paths, coverage of all edge cases
  ```
- The `docs/ci/codecov-rollout.md` "Claim Boundaries" section is the template. Copy it.
- When a doc says "we achieve X%", always pair it with "which means Y is guaranteed and Z is not."

**Durably encoded**: `docs/ci/codecov-rollout.md` "Claim Boundaries" section. This doc hoists the doctrine.

---

## Quick-reference: do-this-first checklist for future autonomous sessions

```
Before reading code to make a forward claim:
  git fetch origin master
  git log --oneline HEAD..origin/master
  (if behind: read from origin/master, not HEAD)

Before spawning a sonnet builder:
  1. Run haiku scout → file GH issue via REST
  2. Builder brief: "Closes #N; preflight done"
  3. PR title: "type(scope): description (#N)"

Before starting a new sub-task:
  - Is this in the last-stated workstream?
  - Is codex/factory-droid already handling this?
  - Can adjacent PRs be batched in the same crate?

Before writing CI config, lint gate, non-rust handler:
  ls policy/ && cat policy/<relevant>.toml
```

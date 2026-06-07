# Builder Brief Template

Sonnet-tier builder brief. Fill in all fields before spawning a builder agent.
Omitting a field wastes a full agent turn on clarification.

Cross-reference:
- [ORCHESTRATION_ROLES.md](ORCHESTRATION_ROLES.md) — builder role constraints
- [EVIDENCE_STANDARD.md](EVIDENCE_STANDARD.md) — required validation artifacts
- [CLOSE_PROOF_POLICY.md](CLOSE_PROOF_POLICY.md) — if closing anything as superseded

---

## Brief Fields

```
Repo:            EffortlessMetrics/perl-lsp-swarm
Branch:          <impl/NNNN-slug or docs/slug — must already exist>
Objective:       <ONE sentence. One PR. One concern.>

Claim boundary:  <What you are NOT doing. Explicit non-goals prevent scope drift.>

Expected files:  <List the files this change touches. Anything else is out of scope.>

Non-goals:
  - <explicit non-goal 1>
  - <explicit non-goal 2>

Known risk:      <Pre-identified edge case, platform quirk, or blocked dependency.>

Required validation (run these exact commands before pushing):
  ./scripts/cargo-safe test -p <crate> --profile agent --locked
  ./scripts/cargo-safe clippy -p <crate> --profile agent --locked -- -D warnings -A missing_docs
  ./scripts/cargo-safe xtask fmt
  just agent-pr-fast

Cleanup:
  - Remove task-owned target/ directories after test run
  - Run ./scripts/storage-doctor and confirm no large repo-local target/
  - Delete task branch locally after PR is merged

Expected PR body:
  Problem: <one sentence>
  Fix:     <one sentence>
  Verification: `cargo test -p <crate>` passes / `just pr-fast` passes
  Dependency: <PR/issue blocking this, or "none">
```

---

## Appendix — Known Gotchas for Builders

The following patterns have caused incidents. Read before pushing.

### 1. Push via HEAD in worktrees

Inside a linked worktree, `git push origin <branch>` may not work as expected
if the local branch name differs from the remote tracking name. Always push via:

```bash
git push origin HEAD:<branch-name>
```

This is safe regardless of worktree state and avoids "refused non-fast-forward" errors.

### 2. Feature-gated tests need parallel `--lib` coverage

Tests behind `#[cfg(feature = "...")]` are invisible to `cargo test --all-targets`
without explicit feature flags. When your change touches feature-gated code:

```bash
# Run both with and without the feature
./scripts/cargo-safe test -p <crate> --profile agent --locked
./scripts/cargo-safe test -p <crate> --profile agent --locked --features <feature-name>
```

A test suite that passes without a feature but fails with it constitutes a
regression even if it was not caught by the default gate.

### 3. Claim-guarded files require running the guard before deletion

Before removing any file or function marked as "dead code", "unused", or "no
references found", run the relevant guard and paste the output:

```bash
just dead-code          # Dead code report
cargo machete           # Unused dependency check
```

Claim-only deletions (no guard output) are rejected by reviewer-deep. See
[QUEUE_CONVERGENCE_DOCTRINE.md](../reference/QUEUE_CONVERGENCE_DOCTRINE.md) Rule 3.

### 4. Arm-then-verify auto-merge state

When enabling auto-merge on a PR:

```bash
gh pr merge --auto --squash <PR>
```

Verify it was armed immediately after:

```bash
gh pr view <PR> --json autoMergeRequest
```

Auto-merge silently fails if the PR is in draft state, has a missing required
label, or the branch protection ruleset blocks squash. Arm-then-verify prevents
the "I thought it would auto-merge" class of stuck PRs.

### 5. Windows `core.longpaths` requirement

On Windows, file paths in nested worktrees can exceed the 260-character limit.
If `git` or `cargo` operations fail with path-related errors:

```bash
git config --global core.longpaths true
```

This must be set in the global git config, not per-repo. See
[docs/project/FRICTION_LOG.md](../project/FRICTION_LOG.md) for full platform context.

### 6. Banned patterns enforced by CI

The following are banned in all production code and will fail clippy/CI:

```
panic!()    unwrap()    expect()    todo!()    unimplemented!()
dbg!()      println!()  eprintln!()  (in library code)
```

Tests must return `Result<()>` or use `perl_tdd_support::must`/`must_some`.
See [docs/NO_PANIC_POLICY.md](../NO_PANIC_POLICY.md) for exceptions.

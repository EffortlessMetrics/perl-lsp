# Fire-Fix Cascade Methodology

**Status:** Pattern derived from the 2026-04-23/04-24 master-unblock sequence where 9+ fire-fix waves stacked on a single PR before master compiled clean.

## When this happens

The symptom: master goes red after a CI scope expansion (e.g., tier-wiring widens `cargo check --lib` to `cargo check --workspace --all-targets`). Fixing the first exposed break surfaces the next one. That surfaces the next. And so on. Each fix uncovers accumulated debt that narrower CI had been hiding.

## The 2026-04-23 cascade in order

Tier-wiring expansion exposed these in sequence. Each discovery required the previous fix to have landed:

1. **`scope_and_symbol_tests.rs:1737`** — `${Foo::name}` literal inside an `assert!` message parsed as a format string. Introduced by merged PR #5090 weeks earlier; `cargo check --lib` never touched the `--tests` target.

2. **`mojolicious_navigation_tests.rs:417`** — stray duplicate `Ok(())\n}` from a merge-conflict resolution in PR #5288. Same reason invisible.

3. **`xtask/lsp_stats.rs`** — incomplete refactor from PR #5303 left `last_run`, `run`, and `load_last_run` undefined at several call sites. `cargo check` on master with tier-wiring revealed it.

4. **Same file, fmt drift** — lines 421, 430 multi-line `assert!` blocks not matching rustfmt's preferred form.

5. **`hash_key_bareword_tests.rs`** — 22 type errors where `&Node` was being passed to a `DiagnosticsProvider::new(ast: &Arc<Node>, ...)` signature. API drift in the production code hadn't been propagated to the tests.

6. **`perl-regex/tests/comprehensive_unit_tests.rs:366`** — function signature formatting drift.

7. **`perl-workspace-index/src/workspace/workspace_index.rs`** — another signature formatting drift.

8. **`perl-semantic-analyzer/tests/scope_and_symbol_tests.rs`** — three long test signatures in a cluster.

9. **`perl-dap/src/platform/mod.rs`** — format drift.

10. **`editor_ux_fixture_matrix.json`** — three scenario-18 workflows missing the `confidence_signals` field a fixture validator required.

11. **`xtask/src/tasks/ci_scope.rs`** — widener rule referenced `perl-lsp-definition`, `perl-lsp-references`, `perl-lsp-rename`, `perl-lsp-workspace` — all crates that had been collapsed into `perl-lsp-rs` in the microcrate-consolidation wave.

12. **`parser_tests.rs:163`** — post-merge of PR #5395 left a long signature rustfmt wanted split. Discovered when the next round of CI ran after #5501 landed.

13–14. Post-merge of PRs #5465 and #5467 left fmt drift in `incremental/mod.rs` and `incremental/incremental_v2.rs` respectively. Discovered over subsequent days as the ratchet kept tightening.

Each step was work in its own right; none was individually visible before the previous ones landed.

## The methodology

### 1. Stack the fixes on one PR

Master can't merge broken PRs. Each fix is individually tiny (1-5 lines). The cheapest path is to stack them as a single PR:

```bash
gh pr checkout <fire-fix-pr>
# apply fix 1; git commit
# discover fix 2 from next CI run; apply; git commit
# ...
# git push --force-with-lease HEAD:refs/heads/<branch>
```

Each iteration: push, wait for CI, read the new failure, apply the next fix. Typically 8-12 minutes per cycle.

Alternative: open a separate PR per fire-fix and serialize-merge. Cost: each PR needs its own review gate. Not worth it unless one of the fixes is large or contentious.

### 2. Don't relax the gate

The instinct when a PR starts failing "noisily" is to suppress the check. Resist it. The noise IS the finding — it's accumulated debt surfacing. Relaxing the gate pushes the debt forward and hides new arrivals.

Tier-wiring is an insurance policy: you pay the cost of the cascade once, then you can trust the widened CI forever.

### 3. Separate pre-existing failures from this-PR failures

During a cascade, most failures are pre-existing master debt, not caused by the PR you're fixing. Document them as pre-existing in the PR body:

> Known pre-existing failures (not caused by this PR):
> - UX Regression Gate (tracked in #5097 — runner contention 30s timeout)
> - Windows Guardrails (module-separator-regressions) (tracked in #5593 — 8.3 short-path)

Ops can then merge on the agreed-set-of-checks rather than all-green.

### 4. Log each fire-fix wave

Every commit on the fire-fix PR should have a distinct, specific message:

- `fix(tests): remove stray duplicate close in mojolicious_navigation_tests`
- `fix(xtask): restore load_last_run helper + fix lsp_stats compile`
- `fix(fmt): split long test fn signature in workspace_index_tests`

This creates a public record of where the debt was hiding. Future scouts reading the commit log can grep for similar patterns.

### 5. File each surfaced issue for separate follow-up

Fire-fix is about unblocking master NOW. It's not the place to deeply investigate root causes. Each discovered break should spawn an issue:

- Why did `--lib`-only CI hide this?
- How did the original PR merge?
- What's the cleanup upstream?

Then the narrow fire-fix lands and the deeper follow-up gets its own triage pass.

### 6. When the cascade won't settle

If fixes keep uncovering new fixes after 10+ iterations, consider:
- Parallel investigation in a different worktree — read all failing jobs at once, don't wait for serial discovery
- `cargo check --workspace --all-targets` locally if the host supports it; the serial CI discovery is slow
- Admin-override merge with documented remaining failures, then chase them in follow-ups

## Post-cascade hygiene

After the cascade resolves, master is now under a tighter contract than before. Expect a flurry of "PR was mergeable yesterday, now shows scope drift" — these are stale-base artifacts, not real regressions. See `BROAD_SCOPE_LAYER_DIVERSITY.md` for the related three-dot-diff triage.

Plan for 1-2 rounds of `gh pr update-branch` sweeps on the whole queue after a cascade lands. Queue will re-settle within 1-2 CI rounds.

## Push-time prevention

The whole cascade was caused by master-break PRs landing with passing CI because CI ran `--lib` only. Prevention is a push-gate `cargo check --workspace --all-targets --locked` that catches these at commit time. See issue #4507.

Effort: 1-2 hours to wire up. Avoids arbitrary-length fire-fix cascades in the future. Highly recommended.

## Related forensics

- `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md` — the session forensic
- `docs/articles/SESSION_2026_04_24_RETROSPECTIVE.md` — broader session context
- `feedback_tier_wiring_exposes_bitrot.md` — companion memory
- Issue #4507 — the push-gate proposal

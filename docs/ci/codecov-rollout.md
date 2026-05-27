# Codecov Rollout

> **Context**: This document is part of perl-lsp's [Industrialized AI](why-industrialized.md) CI architecture. The choices here are responses to operating at 1000+ PRs/day, not premature optimization.

Tightens Codecov's posture in perl-lsp so it accurately reflects what's
actually uploaded, stays out of branch-protection theater, and remains
useful alongside the other evidence lanes.

> Doctrine: Codecov is **one** evidence lane alongside parser corpus, UX
> tests, `ripr`, mutation, real-Perl oracle, no-panic, file policy, and
> release readiness. It is **not** a release-readiness proof.

## Proof-lane Codecov posture

The proof-enforcement lane supersedes the original non-blocking Codecov rollout posture for PR coverage policy.

Current policy:

- patch `95%` / `0%` is the front-door PR coverage policy;
- project `95%` remains informational during burn-down;
- `xtask/src/` is included so proof-rail CLI code stays visible to coverage;
- per-flag `target` fields are not used because project and patch status blocks own thresholds.

This PR slice aligns Codecov configuration and documentation only. It does not implement workflow enforcement, project-coverage final enforcement, or the `quality-gate` CLI.

## What Codecov answers (and doesn't)

Codecov answers:

> Did tests execute this scoped Rust surface, and did branch coverage
> regress beyond the accepted budget?

Codecov does **not** answer:

- whether parser semantics are correct,
- whether tree-sitter behavior is correct,
- whether `@INC` / module-resolution is correct,
- whether LSP / DAP behavior is complete,
- whether CPAN corpus coverage is sufficient,
- whether mutation adequacy is strong,
- whether no-panic policy is clean,
- whether release readiness is proven.

## Current proof-lane target

| Surface                 | Current                                                               | Target                                                    |
| ----------------------- | --------------------------------------------------------------------- | --------------------------------------------------------- |
| Patch status            | Codecov patch `95%` / `0%`, blocking                                  | unchanged                                                 |
| Project status          | Codecov project `95%`, informational during burn-down                 | blocking after project coverage reaches target            |
| Coverage flags          | crate-level flags, including `xtask/src/` for proof-rail code         | keep flags inspectable without per-flag status targets    |
| Branch-coverage ratchet | `.ci/coverage-baseline.txt` parser branch ratchet                     | unchanged in this slice                                   |
| Coverage receipt        | not part of this PR slice                                             | later quality-gate slices define receipt freshness checks |
| Test Analytics          | receipt to JUnit upload in PR-fast / gate shards / UX regression lanes | unchanged; documented as **test telemetry**               |

## Historical current vs target

The table below is retained for historical context and is superseded by the proof-enforcement lane policy above.

| Surface                  | Current                                                                                              | Target                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| README badge             | present (`alt="code coverage"`)                                                                      | clearer alt text (`alt="Codecov parser branch coverage"`); MSRV badge synced to 1.95    |
| `codecov.yml`            | broad: 70% project, 75% patch, `if_ci_failed: error`, per-crate `parser` / `lsp` / `lexer` / `dap` / `corpus` flags, PR comments **on** | quiet: informational statuses, single `parser-branch` flag matching real upload, comments **off**, github-checks annotations **off** |
| Coverage workflow        | inline in `.github/workflows/ci-nightly.yml::test-coverage`                                          | (optional, late) dedicated `.github/workflows/coverage.yml`                             |
| Coverage flag uploaded   | `parser`                                                                                             | `parser-branch` (matches what's actually scoped + the local baseline)                   |
| Branch-coverage ratchet  | `.ci/coverage-baseline.txt` (50.00% branch / 92.11% line / 1.00% allowed drop / 80.00% target)        | unchanged in PR ladder, calibrated only after several stable runs (PR Cov-8)            |
| Coverage receipt         | absent                                                                                               | `target/coverage/coverage-receipt.json` per run, with claim boundary inlined            |
| Test Analytics           | receipt → JUnit upload in PR-fast / gate shards / UX regression lanes                                | unchanged; documented as **test telemetry**, distinct from coverage                      |
| Policy registration      | `codecov.yml` not in `policy/non-rust-allowlist.toml`                                                | added under `policy/non-rust-allowlist.toml` with `review_after` + `covered_by`         |

## Historical Codecov ladder

The older parser-branch Codecov ladder below is retained as history only. Its non-blocking, label-gated posture is superseded by the proof-enforcement lane for active PR coverage policy.

## PR ladder

Each row is one PR. Branch from clean `origin/master`. Do **not** combine.

| #     | Branch                                  | Title                                                          | Tracking      | Notes                                                                                   |
| ----- | --------------------------------------- | -------------------------------------------------------------- | ------------- | --------------------------------------------------------------------------------------- |
| Cov-1 | `ci/codecov-config`                     | `ci(codecov): quiet and scope coverage statuses`               | #8578         | Replace `codecov.yml`: comments off, informational project/patch, single `parser-branch` flag |
| Cov-2 | `ci/coverage-receipt`                   | `ci(coverage): add parser branch coverage receipt`             | #8582         | `ci-nightly.yml::test-coverage` — change flag to `parser-branch`, harden upload condition (token detection + `continue-on-error`), emit `coverage-receipt.json`, write step summary |
| Cov-3 | `docs/codecov-lane`                     | `docs(ci): document Codecov coverage lane boundary`            | #8586         | Create `docs/ci/codecov.md` with claim boundary; reference from `docs/how-to/COVERAGE.md` if/when that doc exists |
| Cov-4 | `docs/readme-codecov-badge`             | `docs(readme): clarify Codecov badge scope`                    | merged #8541  | `alt="code coverage"` → `alt="Codecov parser branch coverage"`; MSRV badge `1.93` → `1.95` |
| Cov-5 | `ci/codecov-test-analytics-docs`        | `ci(codecov): document receipt-backed test analytics`          | #8588         | Adds a table that separates coverage vs Test Analytics vs branch ratchet (none blocking) |
| Cov-6 | `policy/codecov-files`                  | `policy(ci): register Codecov coverage surfaces`               | #8594         | Add entries for `codecov.yml`, `.github/workflows/ci-nightly.yml`, `.ci/coverage-baseline.txt` to `policy/non-rust-allowlist.toml` |
| Cov-7 | `ci/coverage-workflow` *(optional, late)* | `ci(coverage): extract parser coverage into dedicated workflow` | #8668         | Move `test-coverage` job out of `ci-nightly.yml` into `.github/workflows/coverage.yml`; remove the old job |
| Cov-8 | `ci/codecov-ratchet` *(optional, late)* | `ci(codecov): calibrate parser coverage ratchet`               | #8669         | Only after several stable runs; tune `.ci/coverage-baseline.txt` baseline/drop conservatively |

> Tracking issues filed 2026-05-11. Cross-link added via #8670.

## PR Cov-1 — `codecov.yml` shape

Replace the current file with this template. Tighten coverage scope to the
parser/lexer/AST surface that actually has the branch-coverage ratchet
behind it; everything else (`lsp`, `dap`, `corpus`) is removed until those
get their own measurement story.

```yaml
codecov:
  require_ci_to_pass: false

coverage:
  precision: 2
  round: down
  range: "50...85"

  status:
    project:
      parser:
        target: auto
        threshold: 5%
        informational: true
        flags:
          - parser-branch

    patch:
      parser:
        target: 60%
        threshold: 25%
        informational: true
        flags:
          - parser-branch

comment: false

github_checks:
  annotations: false

flags:
  parser-branch:
    paths:
      - crates/perl-parser/src/
      - crates/perl-parser-core/src/
      - crates/perl-lexer/src/
      - crates/perl-ast/src/
      - crates/perl-ast-v2/src/
      - crates/perl-token/src/
    carryforward: true

ignore:
  - "archive/**"
  - "target/**"
  - "crates/tree-sitter-perl-c/**"
  - "crates/tree-sitter-perl-rs/**"
  - "crates/*/tests/**"
  - "crates/*/benches/**"
  - "crates/*/examples/**"
  - "crates/*/build.rs"
  - "xtask/**"
  - "fuzz/**"
  - "vscode-extension/**"
  - "**/*_generated.rs"
```

## PR Cov-2 — `test-coverage` job changes

Inside `.github/workflows/ci-nightly.yml::test-coverage`:

1. Rename the Codecov flag from `parser` to `parser-branch` (matches what
   the ratchet actually measures, and the new `codecov.yml`).
2. Add token detection so the upload step is a no-op when
   `secrets.CODECOV_TOKEN` is absent (fork PRs, etc.).
3. Use `continue-on-error: true` and `fail_ci_if_error: false` on the
   `codecov-action` step.
4. After `just coverage-branch-gate`, emit
   `target/coverage/coverage-receipt.json` with claim-boundary fields.
5. Upload both `lcov.info` and `coverage-receipt.json` as artifacts.
6. Write a GitHub step summary listing artifact presence and the claim
   boundary in one paragraph.

Pin the `codecov/codecov-action` to the existing SHA pinned in the rest of
the workflow file — do not introduce a new floating tag.

## PR Cov-3 — `docs/ci/codecov.md`

```markdown
# Codecov

Codecov is scoped Rust execution-surface telemetry for perl-lsp.

Current uploaded coverage flag: `parser-branch`

Current coverage scope:
- `perl-parser`
- `perl-parser-core`
- `perl-lexer`
- `perl-ast`
- `perl-ast-v2`
- `perl-token`

The lane answers: "Did tests execute this parser/lexer/AST surface, and
did branch coverage regress beyond the accepted baseline budget?"

It does not answer correctness, completeness, or release readiness — see
`docs/development/RUST_1_95_ROLLOUT.md` and `docs/project/status/` for the
relevant evidence lanes.

The local branch-coverage source of truth is `.ci/coverage-baseline.txt`.
Codecov project/patch statuses are informational until stable data is
available. Codecov comments are disabled to reduce PR noise.

Test Analytics is separate from coverage. CI receipts are converted to
JUnit and uploaded so gate behavior is visible without rerunning tests
solely for JUnit.
```

## PR Cov-4 — README edits

```diff
- <img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="code coverage" />
+ <img src="https://codecov.io/gh/EffortlessMetrics/perl-lsp/branch/master/graph/badge.svg" alt="Codecov parser branch coverage" />
- <img src="https://img.shields.io/badge/MSRV-1.93-blue" alt="MSRV" />
+ <img src="https://img.shields.io/badge/MSRV-1.95-blue" alt="MSRV" />
```

Add one sentence near the badge or status section:

```markdown
Codecov is scoped parser branch-coverage telemetry, not a release-readiness or
semantic-correctness proof; see [Codecov rollout](docs/ci/codecov-rollout.md).
```

## PR Cov-5 — Test Analytics table

Add to `docs/ci/codecov.md` and/or wherever lane docs live:

| Codecov surface       | Source                                | Meaning                       | Blocking?                  |
| --------------------- | ------------------------------------- | ----------------------------- | -------------------------- |
| Coverage badge        | `lcov.info` from `test-coverage`      | Parser branch coverage trend  | No                         |
| Project/patch status  | Codecov `parser-branch` flag          | Informational coverage status | No                         |
| Test Analytics        | Receipt → JUnit uploads               | CI gate / test result viz.    | No                         |
| Branch ratchet        | `.ci/coverage-baseline.txt` + script  | Local coverage regression gate | Yes inside coverage lane  |

## PR Cov-6 — Policy registration

Add entries to `policy/non-rust-allowlist.toml`:

```toml
[[allow]]
id = "non-rust-codecov-config"
glob = "codecov.yml"
kind = "ci_coverage_config"
language = "yaml"
surface = "ci"
classification = "config"
owner = "release/ci"
reason = "Configures scoped Codecov parser branch coverage and Test Analytics behavior."
covered_by = ["cargo xtask check-file-policy", "docs/ci/codecov.md"]
created = "2026-05-11"
review_after = "2026-08-11"

[[allow]]
id = "non-rust-ci-nightly-coverage"
glob = ".github/workflows/ci-nightly.yml"
kind = "ci_workflow"
language = "yaml"
surface = "ci"
classification = "config"
owner = "release/ci"
reason = "Runs label-gated and scheduled coverage, mutation, performance, memory, and strict lanes."
covered_by = ["cargo xtask check-file-policy", "docs/ci/codecov.md"]
created = "2026-05-11"
review_after = "2026-08-11"

[[allow]]
id = "non-rust-coverage-baseline"
glob = ".ci/coverage-baseline.txt"
kind = "coverage_baseline"
language = "text"
surface = "ci"
classification = "generated-policy-snapshot"
owner = "release/ci"
reason = "Stores accepted parser branch coverage baseline and regression budget."
covered_by = ["just coverage-branch-gate", "scripts/check-coverage-baseline.sh"]
created = "2026-05-11"
review_after = "2026-08-11"
```

Adjust field names to match what the policy schema actually validates;
this is the intent, not the bit-exact representation.

## PR Cov-7 — Optional dedicated workflow

Skip this PR if `ci-nightly.yml::test-coverage` continues to be ergonomic.
Extract into `.github/workflows/coverage.yml` only if:

- Coverage cadence diverges from "nightly" (PR-label use grows, badge
  consumers want a clean run URL).
- Or workflow file-size becomes a review burden.

When extracting:

- Use `cancel-in-progress: ${{ github.event_name == 'pull_request' && github.event.action == 'synchronize' }}`.
- Trigger on `schedule` + `workflow_dispatch` + PR labels (`ci:coverage`,
  `coverage`, `full-ci`).
- Remove the `test-coverage` job from `ci-nightly.yml` in the same PR.

## PR Cov-8 — Optional ratchet calibration

Only after several stable `master` and `ci:coverage` runs with the new
`parser-branch` flag. Update `.ci/coverage-baseline.txt`:

- Raise `baseline_branch_coverage` only when actuals are consistently above
  it across 5+ runs.
- Lower `allowed_drop_percentage` only when noise is empirically low.
- Do **not** jump straight to the 80% long-term target.

## Acceptance gates (every PR)

```bash
# YAML parse
python3 -c "import yaml; yaml.safe_load(open('codecov.yml').read())"

# Coverage lane locally
just coverage-branch-gate
python3 -m json.tool target/coverage/coverage-receipt.json   # PR Cov-2 onward
cargo xtask fmt
git diff --check
```

## PR body template

```markdown
## Summary

Adds step N of the perl-lsp Codecov cleanup.

## Current behavior
- Coverage lane:
- Codecov upload:
- Test Analytics:
- Claim boundary:

## CI economics
- Default PR impact:
- Label/manual/schedule impact:
- Branch-protection impact:
- Rollback path:

## Claim boundary

Codecov is scoped parser branch-coverage telemetry plus receipt-backed
Test Analytics. It does not prove parser semantic correctness,
tree-sitter correctness, `@INC` / module-resolution correctness, LSP/DAP
correctness, CPAN corpus adequacy, mutation adequacy, no-panic safety,
or release readiness.

## Validation
- [ ] command
- [ ] command

## Self-review
- Scope matches PR title:
- Files touched are expected:
- No duplicate coverage upload lane:
- Codecov remains non-blocking:
- Codecov comments remain disabled:
- Coverage / Test Analytics distinction preserved:
- Local validation:
- CI status:
- Bot comments addressed:
- Follow-ups:
```

## Do not

- Combine Codecov work with: Rust 1.95 lint cleanup, no-panic baseline,
  file-policy rollout, provider cutover, `@INC` work, dependency bumps.
- Make Codecov branch-protection blocking.
- Enable Codecov PR comments.
- Claim Codecov proves parser semantics, LSP / DAP behavior, `@INC`
  correctness, CPAN corpus adequacy, mutation adequacy, no-panic safety,
  or release readiness.

## References

- `docs/development/RUST_1_95_ROLLOUT.md` — parallel rollout ladder.
- `.ci/coverage-baseline.txt` — source of truth for the local ratchet.
- `scripts/check-coverage-baseline.sh`, `scripts/update-coverage-baseline.sh` — ratchet tooling.
- `.github/workflows/ci-nightly.yml::test-coverage` — current coverage lane.

## Burndown status

This section frames the Codecov rollout in the canonical rail-template
shape so it slots cleanly into the rail index alongside
`docs/development/PERL_ORACLE_RAIL.md`,
`docs/development/FILE_POLICY_RAIL.md`, and
`docs/development/CI_UX_RAIL.md`.

> **Substrate (already built)**: this rollout doc itself (#8539, merged),
> README badge fix (#8541, merged), the `parser-branch` flag scope and
> `.ci/coverage-baseline.txt` ratchet baseline. The original umbrella
> #8508 is closed; the rail tracker is #8635.
> **Connector gap**: scope Codecov's upload + status to parser-branch
> coverage (Cov-1), emit a parser coverage receipt artifact (Cov-2), and
> document the evidence-lane boundary (Cov-3, Cov-5) so Codecov never
> implicitly claims more than it measures.
> **0.14.0 upside**: contributors and reviewers can trust the Codecov
> surface as a narrow, accurate signal — parser branch coverage,
> informational — without confusing it for release-readiness proof or
> for parser semantic correctness.

### Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| Cov-1 — scope flag | #8578 | filed by ladder agent | — | `codecov.yml` shape matches Cov-1 template above |
| Cov-2 — parser coverage receipt artifact | #8582 | filed by ladder agent | — | `cargo xtask coverage-ratchet` emits `target/coverage/coverage-receipt.json` |
| Cov-3 — coverage-lane boundary doc | #8586 | filed by ladder agent | — | `docs/ci/codecov.md` lands with claim boundary |
| Cov-5 — Test Analytics doc | #8588 | filed by ladder agent | — | doc section in `docs/ci/codecov.md` clearly separates coverage vs Test Analytics |
| Cov-6 — policy registration | #8594 | filed by ladder agent | — | `cargo xtask check-file-policy --mode advisory` shows `codecov.yml` registered |

Optional, deferred until several stable runs:

| Phase | Issue | Notes |
|---|---|---|
| Cov-7 — extract dedicated workflow | not yet filed | only after Cov-1/2 are stable |
| Cov-8 — calibrate ratchet | not yet filed | only after several stable runs |

### Exit criteria

- [ ] All Cov-* phases land or are explicitly deferred with a successor.
- [ ] Receipt commands in this doc reproduce the closeout proof.
- [ ] Status doc updated (`docs/project/status/index.md`).
- [ ] Claim boundary recorded (this section).

### Claim boundary

**This rail proves**: Codecov is a *scoped, quiet, informational*
evidence lane that reports parser branch coverage against the local
ratchet. Cov-1/2 narrow what's uploaded; Cov-3/5 document what it means;
Cov-6 brings the config file under the same file-policy ledger every
other non-rust surface uses.

**This rail does NOT prove**:

- parser semantics are correct,
- tree-sitter behavior is correct,
- `@INC` / module-resolution is correct,
- LSP / DAP behavior is complete,
- CPAN corpus coverage is sufficient,
- mutation adequacy is strong,
- no-panic policy is clean,
- release readiness is proven.

Those are the work of parser corpus, UX tests, `ripr`, mutation,
real-Perl oracle, no-panic, file policy, and release-readiness lanes
respectively. Codecov is **one** evidence lane among many; treating it
as a release-readiness proof or a semantic-correctness proof is exactly
the failure mode this rail exists to prevent.

### Receipts

```bash
# Branch-coverage ratchet (local, primary receipt).
cargo xtask coverage-ratchet

# PR-fast gate receipt (records what was actually verified pre-merge).
cargo xtask gates --tier pr-fast --base origin/master --receipt

# Confirm Codecov surface is registered under file policy (Cov-6).
cargo xtask check-file-policy --mode advisory

# Per-phase issue status.
gh issue view 8578
gh issue view 8582
gh issue view 8586
gh issue view 8588
gh issue view 8594
```

### Related

- Umbrella issue: #8635 (rail tracker — the original #8508 is closed and serves only as historical context).
- Architecture / spec docs: this file (`docs/ci/codecov-rollout.md`); Cov-3 will add `docs/ci/codecov.md`.
- Status doc: `docs/project/status/index.md`.
- Adjacent rails:
  - `docs/development/FILE_POLICY_RAIL.md` — Cov-6 depends on the non-rust allowlist tooling landing first.
  - `docs/development/CI_UX_RAIL.md` — the PR sticky summary will reference coverage status; the contract between them is that sticky owns "what ran" and Codecov stays informational.
  - `docs/development/PERL_ORACLE_RAIL.md` — Perl-oracle tests are not in the Codecov-scoped surface; coverage there is intentionally out of scope.

### Do not combine

- Do not combine Codecov work with: Rust 1.95 lint cleanup, no-panic
  baseline, file-policy strict-mode promotion (Cov-6 lands *after*
  advisory mode is stable), provider cutover, `@INC` work,
  Perl-oracle migrations, dependency bumps.
- Do not make Codecov branch-protection blocking.
- Do not enable Codecov PR comments.
- Do not claim Codecov proves parser semantics, LSP / DAP behavior,
  `@INC` correctness, CPAN corpus adequacy, mutation adequacy,
  no-panic safety, or release readiness.

### Lane assignment

**codex** owns the Cov-* PRs. The CI-economics lane is codex /
factory-droid territory by convention; the CI-economics ladder agent
(`ac0d0d6984fa31b60`, running at file-time) is filing the Cov-* rows.
Coordinate by searching open issues with the `codecov` or `Cov-`
filename pattern before filing duplicates:

```bash
gh api 'repos/EffortlessMetrics/perl-lsp/issues?state=open&per_page=100' \
  --jq '.[] | select(.title | test("(codecov|Cov-)")) | "#\(.number) \(.title)"'
```

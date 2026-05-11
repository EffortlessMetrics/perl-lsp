# Codecov Rollout

Tightens Codecov's posture in perl-lsp so it accurately reflects what's
actually uploaded, stays out of branch-protection theater, and remains
useful alongside the other evidence lanes.

> Doctrine: Codecov is **one** evidence lane alongside parser corpus, UX
> tests, `ripr`, mutation, real-Perl oracle, no-panic, file policy, and
> release readiness. It is **not** a release-readiness proof.

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

## Current vs target

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

## PR ladder

Each row is one PR. Branch from clean `origin/master`. Do **not** combine.

| #     | Branch                                  | Title                                                          | Notes                                                                                   |
| ----- | --------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Cov-1 | `ci/codecov-config`                     | `ci(codecov): quiet and scope coverage statuses`               | Replace `codecov.yml`: comments off, informational project/patch, single `parser-branch` flag |
| Cov-2 | `ci/coverage-receipt`                   | `ci(coverage): add parser branch coverage receipt`             | `ci-nightly.yml::test-coverage` — change flag to `parser-branch`, harden upload condition (token detection + `continue-on-error`), emit `coverage-receipt.json`, write step summary |
| Cov-3 | `docs/codecov-lane`                     | `docs(ci): document Codecov coverage lane boundary`            | Create `docs/ci/codecov.md` with claim boundary; reference from `docs/how-to/COVERAGE.md` if/when that doc exists |
| Cov-4 | `docs/readme-codecov-badge`             | `docs(readme): clarify Codecov badge scope`                    | `alt="code coverage"` → `alt="Codecov parser branch coverage"`; MSRV badge `1.93` → `1.95` |
| Cov-5 | `ci/codecov-test-analytics-docs`        | `ci(codecov): document receipt-backed test analytics`          | Adds a table that separates coverage vs Test Analytics vs branch ratchet (none blocking) |
| Cov-6 | `policy/codecov-files`                  | `policy(ci): register Codecov coverage surfaces`               | Add entries for `codecov.yml`, `.github/workflows/ci-nightly.yml`, `.ci/coverage-baseline.txt` to `policy/non-rust-allowlist.toml` |
| Cov-7 | `ci/coverage-workflow` *(optional, late)* | `ci(coverage): extract parser coverage into dedicated workflow` | Move `test-coverage` job out of `ci-nightly.yml` into `.github/workflows/coverage.yml`; remove the old job |
| Cov-8 | `ci/codecov-ratchet` *(optional, late)* | `ci(codecov): calibrate parser coverage ratchet`               | Only after several stable runs; tune `.ci/coverage-baseline.txt` baseline/drop conservatively |

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
semantic-correctness proof; see [Codecov](docs/ci/codecov.md).
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

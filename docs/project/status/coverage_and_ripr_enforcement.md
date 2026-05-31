# Coverage and RIPR Enforcement

> Human-owned. Update when the proof-lane policy or transition exceptions
> change. Do not use this page to claim final enforcement before the gates are
> blocking in CI.

## Current Blocking Proof Floor

The proof lane is in transition from measurement to enforcement:

- GitHub branch protection requires the current proof-floor contexts:
  `ripr+ New Gap Gate`, `Codecov / Patch 95`, `codecov/patch`, and
  `Perl LSP Rust Small Result`
- `Perl LSP Rust Small Result` must pass before merge
- `ripr+ New Gap Gate` blocks new RIPR gaps and stale or missing RIPR proof
  receipts
- `Codecov / Patch 95` blocks patch coverage below 95%, stale or missing
  coverage receipts, missing coverage artifacts, and Codecov upload or
  processing failures through `fail_ci_if_error: true`
- PR patch coverage is routed by changed surface through `cargo xtask ci route`;
  code routes run the focused coverage pack selected from
  `.ci/coverage-packs.toml`, while routes without a coverage pack fall back to
  the broader workspace proof so the required Codecov status is still produced
- `codecov/patch` must complete and pass after Codecov processes the uploaded
  LCOV
- generated quality-gate receipts are freshness-checked for patch, new-RIPR,
  and final modes; the final mode remains a future enforcement flip until
  burn-down closes
- CI uploads required RIPR and coverage proof artifacts and appends quality-gate
  Markdown summaries to the GitHub job summary

This is the current merge contract. A PR with pending, failed, missing, skipped
unexpectedly, or stale required proof contexts is not merge-ready.

## Active RIPR+ Inventory

The repo-wide RIPR+ baseline is generated with:

```bash
rtk cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json
rtk cargo xtask ripr-plus --receipt target/receipts/quality/ripr-plus.json --check
```

The receipt is the burn-down inventory source. It keeps the existing
compatibility fields (`unresolved`, `suppressed`, `top_files`) and also reports
the explicit queue-shaping fields:

- `active_unresolved`
- `suppressed_unresolved`
- `top_active_files`
- `top_active_gap_kinds`
- `top_suppressed_files`
- `top_suppressed_gap_kinds`
- `suppressions.reasons`
- `recommended_first_clusters`

This inventory is measurement only. It separates active proof debt from
suppressed archive/generated/non-production surfaces and recommends reviewable
clusters for future burn-down PRs. It does not start parser, semantic, lexer, or
LSP behavior burn-down by itself.

## Project Coverage Burn-Down Inventory

The workspace coverage baseline is generated with:

```bash
rtk cargo xtask coverage-baseline --lcov target/lcov.info --receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml --patch-base origin/HEAD --scope routed-coverage-packs
rtk cargo xtask coverage-baseline --lcov target/lcov.info --receipt target/receipts/quality/coverage-baseline.json --codecov codecov.yml --patch-base origin/HEAD --scope routed-coverage-packs --check
```

The receipt keeps patch-gate fields for enforcement and also reports the
project burn-down queue:

- `coverage.project`
- `project_burndown.target`
- `project_burndown.current`
- `project_burndown.remaining_percentage_points`
- `project_burndown.status`
- `project_files_below_target`
- `top_project_files`
- `recommended_project_clusters`

This inventory is queue-shaping only while project coverage remains
transitional. Use it to split future coverage PRs by behavior surface:
proof infrastructure, CLI/report generation, provider decisions,
scheduler/cancellation, config parsing, serialization/deserialization, and
error handling. Do not treat the inventory as permission to chase generated code
or add constructor-only line coverage tests.

## Transitional Targets

- project coverage is visible in coverage receipts as `coverage.project`, but
  project coverage 95% is not branch-protection blocking yet
- total active RIPR+ unresolved gaps still have to burn down to zero before
  final enforcement can block on repo-wide RIPR+ total
- temporary burn-down exceptions are tracked in
  [`policy/quality-gate-exceptions.toml`](../../../policy/quality-gate-exceptions.toml)

Temporary exceptions are not success criteria. They exist so the transition gate
can block new proof regressions while existing repo-wide debt is burned down.
An active temporary exception is not a final-enforcement pass.

## Exception Contract

Every temporary quality exception must name:

- `kind = "temporary_burndown"`
- `scope`
- `owner`
- `reason`
- `final_target`
- `evidence`
- `removal_criteria`
- `review_after`
- `expires`

Expired exceptions fail the quality gate. Due-for-review exceptions follow the
ledger's `due_review` policy. Active exceptions are reported as
final-enforcement blockers until their removal criteria are satisfied and the
exception is removed.

## Active Burn-Down Exceptions

| ID | Scope | Final target | Removal signal |
|---|---|---|---|
| `ripr-total-burndown` | `ripr_plus_total` | repo-wide ripr+ unresolved total = 0 | final quality gate requires total zero |
| `project-coverage-burndown` | `project_coverage` | workspace project coverage >= 95% | Codecov project coverage is blocking at target |

## Non-Goals

This policy does not weaken the patch coverage or new RIPR gap gates. It also
does not authorize broad refactors, LSP 3.18 protocol implementation, or
coverage tests that only chase lines without proving behavior.

# Coverage and RIPR Enforcement

> Human-owned. Update when the proof-lane policy or transition exceptions
> change. Do not use this page to claim final enforcement before the gates are
> blocking in CI.

## Current Posture

The proof lane is in transition from measurement to enforcement:

- new RIPR gaps are enforced by `cargo xtask quality-gate --mode enforce-new-ripr`
- patch coverage is enforced by `cargo xtask quality-gate --mode enforce-patch-coverage`
- GitHub branch protection requires the current proof-floor contexts:
  `ripr+ New Gap Gate`, `Codecov / Patch 95`, `codecov/patch`, and
  `Perl LSP Rust Small Result`
- CI uploads required RIPR and coverage proof artifacts and appends quality-gate
  Markdown summaries to the GitHub job summary
- repo-wide RIPR+ zero and project coverage 95% remain burn-down targets
- temporary burn-down exceptions are tracked in
  [`policy/quality-gate-exceptions.toml`](../../../policy/quality-gate-exceptions.toml)

Temporary exceptions are not success criteria. They exist so the transition gate
can block new proof regressions while existing repo-wide debt is burned down.

## Exception Contract

Every temporary quality exception must name:

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

| ID | Final target | Removal signal |
|---|---|---|
| `ripr-total-burndown` | repo-wide ripr+ unresolved total = 0 | final quality gate requires total zero |
| `project-coverage-burndown` | workspace project coverage >= 95% | Codecov project coverage is blocking at target |

## Non-Goals

This policy does not weaken the patch coverage or new RIPR gap gates. It also
does not authorize broad refactors, LSP 3.18 protocol implementation, or
coverage tests that only chase lines without proving behavior.

# CI Labels

Label vocabulary for routing and budget acknowledgement. Labels are advisory until the
PR Plan workflow lands (PR 04) and budget guard is active (PR 13). Naming aligns with
existing `ci:*` labels already used in some workflows.

> Companion: [cost-and-verification-policy.md](cost-and-verification-policy.md),
> [lem-budgeting.md](lem-budgeting.md).

---

## Budget labels

| Label | Meaning |
|---|---|
| `full-ci` | Run broad validation beyond the ordinary PR gate. Implies budget acknowledgement. |
| `ci-budget-ack` | Acknowledge an elevated CI spend (35–125 LEM band). |
| `ci-budget-override` | Override the hard 125 LEM ceiling. Use sparingly; always state reason in PR body. |

## Lane-trigger labels

| Label | Meaning |
|---|---|
| `ripr` | Force `ripr` static exposure analysis. |
| `ripr-waive` | Acknowledge a `ripr` advisory finding (only after PR 18). |
| `ci:coverage` / `coverage` | Run coverage lane. |
| `ci:mutation` / `mutation` | Run runtime mutation testing. |
| `ci:perl-matrix` | Run Perl version matrix lane. |
| `ci:vscode-matrix` | Run VS Code OS matrix smoke. |
| `ci:memory` | Run memory plateau workload. |
| `ci:bench` / `ci:real-repo-latency` | Run real-repo latency / bench lanes. |
| `ci:parser` | Force parser-related lanes. |
| `ci:ux` | Force UX regression lane. |
| `ci:dap` | Force DAP regression lanes. |
| `ci:security` / `security-audit` | Force audit / deny / Trivy lane. |
| `release-check` | Run release/package dry-run lanes. |

## Out-of-scope labels (informational)

| Label | Meaning |
|---|---|
| `ai-review` | Force external AI review on draft / non-credible PRs. |

---

## Label hygiene rules

- One label per intent.
- Adding a `ci:*` lane label without a reason in the PR body is discouraged.
- `ci-budget-override` requires an explicit dollar/LEM reason.
- Suppressions (e.g. `ripr-waive` on a finding) require an entry in the relevant
  suppressions ledger (e.g. [`policy/ripr-suppressions.toml`](../../policy/ripr-suppressions.toml))
  with `owner`, `reason`, `created`, `review_after`, `expires`.

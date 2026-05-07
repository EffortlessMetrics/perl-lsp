# Workflow Policy Lint

`cargo xtask workflow-policy-lint` validates every workflow under
`.github/workflows/` against a set of policy rules. It is the existing
mechanism for catching unsafe `pull_request_target` patterns, write-all
permissions, blanket cancel-in-progress, and similar foot-guns.

PR 11 of the CI economics rollout extends the linter with an opt-in
**lane-whitelist** check.

> Companion: [policy-ledgers.md](policy-ledgers.md), [perl-lsp-rollout-plan.md](perl-lsp-rollout-plan.md).

---

## Existing rules (errors unless noted)

| Code | Meaning |
|---|---|
| `PR_TARGET_CHECKOUT_HEAD` | `pull_request_target` workflow checks out PR head (unsafe) |
| `WRITE_ALL_PERMISSIONS` | `permissions: write-all` declared |
| `PR_CONTENTS_WRITE` | `pull_request` workflow requests `contents: write` (not allowlisted) |
| `UNTRUSTED_PR_SECRETS` | Untrusted PR code path consumes `secrets.*` |
| `REQUIRED_STYLE_MISSING_MERGE_GROUP` | Required-style workflow missing `merge_group` trigger |
| `REQUIRED_STYLE_SELF_FILTERED` | Required-style workflow path-filters itself |
| `BLANKET_CANCEL_IN_PROGRESS` | `cancel-in-progress` not gated for master/merge_group truth runs |
| `LABEL_EVENT_CANCELS_PR_RUN` | `pull_request labeled`/`unlabeled` workflow cancels in-progress runs |
| `UNPINNED_ACTION` | Third-party action not pinned to a commit SHA (warning) |

---

## New: `LANE_WHITELIST_MISSING` (advisory)

```bash
cargo xtask workflow-policy-lint --check-lane-whitelist
```

For each workflow under `.github/workflows/*.yml`, the linter checks whether
[`policy/ci-lane-whitelist.toml`](../../policy/ci-lane-whitelist.toml) has at
least one `[[lane]]` entry whose `workflow` field matches that file.

If neither a whitelist entry nor an `ALLOWLIST_WORKFLOW_LANE_MISSING`
allowlist entry covers the workflow, the linter emits a **warning** with
code `LANE_WHITELIST_MISSING`.

The check is **advisory only** — warnings, not errors. The `passed` status
in the receipt is unaffected. Promotion to error level is intentionally
deferred until the whitelist has stabilized.

### Allowlist

`ALLOWLIST_WORKFLOW_LANE_MISSING` in
[`xtask/src/tasks/workflow_policy_lint.rs`](../../xtask/src/tasks/workflow_policy_lint.rs)
covers release/utility workflows that are not part of the per-PR economics
map. Initial entries:

- Release/publish: `release.yml`, `publish-*.yml`, `docker-publish.yml`,
  `*-bump.yml`, `release-orchestration.yml`, `post-publish-smoke.yml`,
  `vscode-published-extension-smoke.yml`
- Post-merge utilities: `post-merge-corpus-ratchet.yml`,
  `post-merge-status.yml`, `docs-deploy.yml`
- Schedule-only / housekeeping: `tokmd.yml`, `merge-ready-reconciler.yml`,
  `triage-issues.yml`, `version-bump.yml`, `ci-gate-self-tests.yml`,
  `workflow-trigger-lint.yml`

To add a new workflow that should be exempt, add a constant entry there and
explain the reason in the commit message.

---

## When to enable in CI

The lane-whitelist check is currently invoked only when explicitly passed
the flag. To enable it on PRs, add to `.github/workflows/workflow-policy.yml`:

```yaml
- name: Run workflow policy lint
  run: cargo xtask workflow-policy-lint --check-lane-whitelist --receipt target/receipts/workflow-policy.json
```

Recommended approach: leave advisory-only first, observe the warning rate
on real PRs, then promote to error once the whitelist + allowlist are
stable.

---

## Output

Warnings appear as `::warning::` annotations in the GitHub workflow log.
The receipt JSON includes them in `issues[]` with `level: "warning"`,
`code: "LANE_WHITELIST_MISSING"`. The `passed` field of the receipt is
unaffected by warning-level issues.

# CI Architecture Reference

> For the umbrella system design — what an Octopus Cluster is, how CI fits into the
> candidate-to-trusted-change pipeline, and shared vocabulary — see
> [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md).
> For operational failure patterns in CI, see [FAILURE_MODES.md](FAILURE_MODES.md).
> For the live-signal vs label distinction that governs merge-readiness decisions, see
> [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md).

---

## Overview

This document is the formal reference for the CI tiering used by the perl-lsp Octopus
Cluster. It answers:

- What runs where and when?
- What does "required" vs "informative" mean?
- How does `ci-scope` select lanes?
- What is a receipt and how do agents use it?
- How does live CI differ from the `ci-green` label?
- Why does the cancellation cascade exist and what mitigates it?
- What gaps exist today and what roadmap items address them?

**Sources of truth:**
- Gate definitions: `.ci/gate-policy.yaml`
- Workflow runner: `.github/workflows/ci.yml`
- Scope classifier: `xtask/src/tasks/ci_scope.rs`
- Gate runner: `xtask/src/tasks/gates.rs`
- Receipt schema: `.ci/receipt.schema.json`

---

## Section 1 — The Three Tiers

CI is organized into three conceptual tiers. The tiers correspond to different points in
the candidate lifecycle and have very different economics.

### Tier 1: Frontdoor Proof

**What it is**: The first CI pass on every credible candidate PR.

**Purpose**: Mechanical credibility check — broken compile, obvious test failures, format
violations, and policy invariants. Establishes whether a candidate is worth curating at
all.

**Target duration**: 10–20 minutes total (pr-smoke ~1–2 min, merge-gate ~5–10 min).

**When it runs**: On every push to a non-draft PR. Also on push to master.

**Who uses it**: Curators (selecting winners from ensembles), PR followers (verifying that
CI hasn't broken since their last look).

**Workflow jobs** (from `.github/workflows/ci.yml`):

| Job | Timeout | Scope |
|-----|---------|-------|
| `pr-smoke` | 35 min | Format, scoped clippy, scoped test, integration-test compile |
| `merge-gate-shards` | 20 min | Bounded merge-gate shards via gate policy (see Section 2) |
| `merge-gate` | 2 min | Aggregates shard results into the required merge-blocking status |
| `ux-tests` | 15 min | UX regression suite against live binary |
| `check-all-targets` | 10 min | `cargo check --workspace --all-targets` (bit-rot guard) |
| `windows-guardrails` | 20 min | Three Windows-specific lanes (compile, module-sep, sandbox) |

**Scoping within Frontdoor Proof**: The pr-smoke job uses `cargo xtask ci-scope` to
classify the diff and select scoped lanes. This is "scoped-deep" CI — narrow to the blast
radius of the change, but thorough within that scope. See Section 3 for the full scoping
logic.

**Key property**: Runs on candidates before curation. A PR with a hallucinated API fails
`cargo check` in ~2 minutes and is closed cheaply. The alternative — running expensive
survivor-level checks on every candidate — would cost 10–50x more for no additional signal
on candidates that fail basic compilation.

**Example**: During the #7090 fmt cascade response, Frontdoor Proof was what revealed that
12 apparently-unrelated PRs were all failing at the same step (`cargo xtask fmt --check`).
This fingerprint — N unrelated PRs failing identically — is the master bit-rot signal.
The response was: fix master once, cascade the fix to all affected PRs, not investigate
each PR independently. See [FAILURE_MODES.md — Master Bit-Rot Cascade](FAILURE_MODES.md).

---

### Tier 2: Survivor-Level Verification

**What it is**: Expensive checks that run only on curated survivors and high-risk PRs.

**Purpose**: Deep validation that would be cost-prohibitive to run on every candidate.
Verifies test quality (mutation), robustness (fuzzing), real-world parser compatibility
(full corpus), and cross-platform consistency (platform soak).

**Target duration**: Up to 60 minutes; no hard wall for nightly schedule.

**When it runs**: On schedule (3 AM UTC nightly), or promoted by risk tags (see Section 3),
or manually via `workflow_dispatch`. Not on every PR push.

**The nightly tier** (`tier: nightly` in `.ci/gate-policy.yaml`):

| Gate | Purpose |
|------|---------|
| `mutation` | `cargo mutants` on `perl-parser-core`; verifies test quality, not just coverage |
| `fuzz` | `proptest` with 2048 cases against `perl-parser` |
| `benchmarks` | Parser performance benchmarks, tracked over time |
| `full_matrix` | Ubuntu + Windows + macOS across stable, MSRV, and beta toolchains |
| `coverage` | `cargo-llvm-cov` coverage against baseline |
| `corpus_validation` | Full corpus audit |
| `corpus_sweep` | Parser corpus sweep ratchet |
| `determinism_check` | Verify test output is deterministic across 3 runs |

**Enforcement**: All nightly gates have `required: false` and `enforcement: informational`.
They produce signal and are tracked over time but do not block PR merges. A nightly gate
failure is a quality signal, not a merge blocker. It should be addressed in a follow-up PR.

**Heavy-lane promotion**: Some survivor-level checks can be promoted to run on a specific
PR if the diff triggers a risk tag. For example, a change to parser recovery code
(`parser_recovery` risk tag) promotes `bounded_parser_fuzz` to run alongside Frontdoor
Proof. See Section 3 for the full risk-tag-to-lane mapping.

**Example**: PR #7031 (master test panic blocker) was caught because the `unit_core` and
`unit_full` gates in the merge-gate tier failed. The test bug — a variable shadow causing
a `tempfile::tempdir()` binding to drop early — would not have been caught by a coverage
check or a mutation check. It required the full test suite to execute. This is why
`unit_full` is a required merge-gate check, not a nightly advisory.

---

### Tier 3: Master Watcher

**What it is**: CI that runs on every push to master (not just PRs).

**Purpose**: Verify trunk health after each merge. A failure here is a master bit-rot
incident — not a PR-side issue.

**When it runs**: On `push: branches: [master]` (enforced in `.github/workflows/ci.yml`
`on:` section). Also on `workflow_dispatch`.

**Key distinction**: Frontdoor Proof runs on PR branches. Master Watcher runs on master
after merge. Both run the same gate commands, but they have different failure semantics:

- **Frontdoor Proof failure**: This PR has a problem. Fix the PR.
- **Master Watcher failure**: Master is broken. Fix master immediately. Do not merge
  additional PRs until the trunk is green.

**Incident response** (see [FAILURE_MODES.md — Master Bit-Rot Cascade](FAILURE_MODES.md)):
1. Identify the failure as master-level (same error across N unrelated PRs, or reproduces
   locally against `origin/master`).
2. Fix master in a narrow PR.
3. Admin-merge once local verification passes — don't wait for full CI on the fix PR.
4. Run `gh pr update-branch` for every blocked PR (propagates the fix to open PRs).
5. Do not try to fix N PRs individually; fix master once.

**Historical examples from this session:**
- #7090: `cargo xtask fmt` rule change broke format check for 12+ PRs simultaneously.
- #7031: Test variable shadow (`tempfile::tempdir()` binding dropped early) blocked the
  queue for ~24 hours, caught via local repro against `origin/master`.

**Gap: `push: branches: [master]` must be on quality-comparison workflows.** Any workflow
that compares a PR against a "master baseline" needs a master push trigger to keep the
baseline current. Without it, the baseline degrades to the regressed state after a bad
merge, and every subsequent PR compares regression-to-regression. This is the
[PR-Only Trigger Observability Gap](FAILURE_MODES.md) failure mode.

---

## Section 2 — Gate Definitions and Policy

All gate definitions live in `.ci/gate-policy.yaml`. The schema version is 1. The
`xtask/src/tasks/gates.rs` runner reads this policy and executes gates, emitting receipts.
`.ci/GATE_REGISTRY.toml` is a legacy metadata index and is **not** used to decide
merge blocking behavior. Use `cargo xtask gate-policy check` to verify policy/registry
alignment and PR-safety invariants (including CPAN non-blocking on PR profile).

### Tier Mapping

| Policy Tier | Trigger | Enforcement | Target Duration |
|-------------|---------|-------------|-----------------|
| `pr_fast` | pull_request | `required` | <3 min |
| `merge_gate` | merge_queue, workflow_dispatch | `required` | <8 min |
| `nightly` | schedule (03:00 UTC) | `informational` | <60 min |
| `release` | workflow_dispatch | `required` | <10 min |

### Required Merge-Gate Checks

The following gates have `required: true` and `tier: merge_gate`. A failure in any of
these blocks merge (via the `ci/merge-gate` commit status check):

| Gate | Command summary | Purpose |
|------|----------------|---------|
| `clippy_full` | `cargo clippy --workspace --lib` + `--bins --no-deps` | Full lint including unwrap/expect ban |
| `unit_full` | `cargo test --workspace --lib --locked` | All workspace library tests |
| `compile_all_targets` | `just check-all-targets` | Catch integration-test and bench bit-rot |
| `lsp_smoke` | `cargo test -p perl-lsp-rs --test semantic_definition` | Deterministic LSP integration test |
| `lsp_tier_a` | CLI smoke + capabilities snapshot + protocol tests | LSP capability correctness |
| `lsp_tier_b` | Definitions, completion, color, code lens, security, behavioral | LSP core behavior |
| `common_corpus_clean` | `xtask parser-corpus-sweep --manifest --enforce` | Common Perl modules parse with zero errors |
| `parser_audit_closeout` | `xtask corpus-audit --fresh --check` | Corpus parser metrics do not regress |
| `security_audit` | `cargo audit --deny warnings` | Known CVEs in dependencies |
| `policy_checks` | Version sync + docs baseline + features invariants | Project policy invariants |
| `docs_build` | `cargo doc -p perl-parser -p perl-lsp-rs` | Documentation builds without errors |
| `v2_parity` | `xtask corpus --scanner v2-parity` | v2 and perl-parser-pest produce identical output |
| `v2_bundle_sync` | `bash scripts/check-v2-bundle-sync.sh` | v2 bundle files stay synchronized |
| `workflow_audit` | `python3 scripts/ci-audit-workflows.py` | No ungated expensive jobs in workflows |
| `nested_lock_check` | `find . -name Cargo.lock` | No nested Cargo.lock files |
| `published_crate_count` | `xtask published-crate-count` | Crate count ratchet |

### Advisory (Informational) Gates

These gates have `required: false`. Failures produce signal and are tracked in
`.ci/debt-ledger.yaml` but do not block merge:

| Gate | Why advisory |
|------|-------------|
| `parser_corpus_ratchet` | Baseline drifts with runner Perl version (Ubuntu Perl updates produce environmental false positives) |
| `cpan_corpus_ratchet` | CPAN corpus not installed on PR runners; owned by post-merge cron |
| `security_audit` | Currently quarantined: `cargo-audit` ecosystem breakage as of 2026-04-26 |
| `published_crate_count` | Quarantined until collapse completes (~30–31 target crates) |
| All `nightly` gates | Informational by tier definition |

**Quarantine tracking**: Quarantined items are tracked in `.ci/debt-ledger.yaml`. The
`policy_checks` gate includes `just debt-check` which fails if debt budgets are exceeded or
quarantines are expired. A gate in quarantine is not forgotten — it is tracked against a
budget and a re-evaluation deadline.

---

## UX Regression pre-merge behavior

The `ux-tests` lane in `.github/workflows/ci.yml` is merge-blocking and always emits:

- `target/receipts/ux-regression.log`
- `target/receipts/ux-regression.json`

The JSON receipt classifies the first observed failure and provides routing metadata and
repro commands. Classification helps routing and triage, but it does **not** make a
failing UX run pass.

| Failure class | Route | Expected action |
| --- | --- | --- |
| `provider_regression` | `provider_fix` | Fix LSP/provider behavior before merge |
| `test_race` | `test_fix` | Stabilize harness or quarantine with a tracked follow-up issue |
| `matrix_drift` | `fixture_update` | Update fixture matrix inputs/expectations |
| `baseline_drift` | `baseline_update` | Regenerate accepted baseline |
| `timeout` | `timeout_triage` | Separate CI slowness from product regression |
| `infra` | `ci_investigation` | Fix CI/harness infrastructure |
| `server_crash` | `crash_fix` | Fix process crash before merge |
| `new_test_bug` | `test_fix` | Fix test logic/regression in scenario |
| `unknown` | `triage` | Inspect receipt log and extend classifier coverage |

---

## Section 3 — Scoping: How `ci-scope` Selects Lanes

The pr-smoke job uses `cargo xtask ci-scope --base origin/master --format json` to
classify the diff and select lanes. Source: `xtask/src/tasks/ci_scope.rs`.

### Diff Classification

Changed files are classified into one of five diff classes. The class determines whether
heavy CI lanes run:

| Class | Triggers | CI effect |
|-------|---------|-----------|
| `prose_only` | `.md`, `.txt`, `.rst`, `docs/` prefix | Skip all Rust build lanes |
| `docs_as_code` | `.toml`, `.yaml`, `.json` (not CI config) | Skip Rust build lanes |
| `ci_config` | `.github/workflows/`, `.ci/`, `scripts/`, `justfile` | Skip Rust build lanes |
| `code` | Rust source files | Full scoped lanes |
| `mixed` | Combination of above | Full scoped lanes (conservative) |

This is the "skip notice" mechanism visible in the pr-smoke job:
```
docs/reference/CI_ARCHITECTURE.md → diff_class=prose_only → skip Rust build lanes
```

### Crate Discovery

For `code` and `mixed` diffs, `ci-scope` maps changed files to crates:

1. Changed file path → `crates/<name>/src/*.rs` → extract crate directory `crates/<name>`
2. Cross-reference against `cargo metadata` to get the canonical package name
3. Output: `direct_crates[]` — the crates directly touched by the diff

### Reverse-Dependency Closure

From the directly-changed crates, `ci-scope` computes the transitive reverse-dependency
closure using the `resolve.nodes` from `cargo metadata`. If crate A is changed and crate B
depends on A, then B is in the reverse-dep closure. The closure is transitive (A → B → C
means C is also included).

Example: A change to `perl-lexer` → direct crate. `perl-parser` depends on `perl-lexer`
→ included in reverse-dep closure. `perl-lsp-rs` depends on `perl-parser` → also included.

The union of direct crates and reverse-dep closure becomes the `-p` arguments to scoped
clippy and scoped test.

### Architectural Wideners

Some crate-relationship boundaries are not captured by direct Cargo dependencies. The three
widener rules in `WIDENER_RULES` express cross-cutting architectural relationships:

| Trigger | Targets added | Lane promoted | Reason |
|---------|--------------|--------------|--------|
| `perl-parser`, `perl-lexer`, `perl-parser-core` | `perl-semantic-analyzer`, `perl-workspace`, `perl-lsp-rs`, `perl-dap` | `lsp_smoke` | Parser changes propagate to LSP/DAP |
| `perl-semantic-analyzer`, `perl-workspace` | `perl-lsp-rs-core`, `perl-lsp-rs` | `lsp_providers` | Semantic changes propagate to LSP providers |
| `perl-lsp-*`, `perl-dap` | `perl-lsp-rs` | `ux_regression` | LSP/DAP changes trigger UX regression check |

### Risk Tags

Beyond crate-level scoping, `ci-scope` detects risk tags from file path heuristics:

| Tag | Detected by | Heavy lane promoted |
|-----|------------|---------------------|
| `parser_recovery` | Path contains `/recovery/` or `/expressions/` | `bounded_parser_fuzz` |
| `concurrency` | Path contains `async`, `concurrent`, `thread`, `mutex`, `rwlock` | `thread_sanitizer` |
| `perf_hot_path` | Path under `benchmarks/`, `benches/`, `criterion/` | `perf_regression` |
| `security_surface` | Path contains `auth`, `eval`, `exec`, `deserializ`, `shell` | `security_audit` |
| `dep_change` | `Cargo.toml` or `Cargo.lock` changed | (infra lanes: `publish`, `security`, `ci_policy`) |
| `public_api` | Direct change to facade crate (`perl-parser`, `perl-lsp-rs`, `perl-dap`, `perl-uri`) | (advisory) |
| `offset_math` | Path contains `position`, `offset`, `utf`, `column` | (advisory) |
| `path_normalization` | Path contains `uri`, `workspace-folder`, `file_uri` | (advisory) |

### Special Path Triggers

| Changed file | Additional lanes |
|-------------|-----------------|
| `features.toml` | `ux_regression` (per #4706) |
| `Cargo.toml`, `Cargo.lock`, `justfile`, `.github/workflows/**`, `hooks/**` | `publish`, `security`, `ci_policy` (workspace-root infra check) |

### Fallback Behavior

If `ci-scope` fails (timeout, cargo metadata error, or other failure), the pr-smoke job
falls back to a hardcoded baseline: `just clippy-core` and `just test-core`. This ensures
a broken `ci-scope` does not produce a false-green PR — it produces a broader (slower)
check rather than no check.

### Timeout Classification

`pr-smoke` has a 35-minute GitHub job timeout and wraps `xtask gates --tier pr-fast`
with a 30-minute runner watchdog. Individual shell-backed gates still use their
`.ci/gate-policy.yaml` `timeout_seconds` values. On Unix runners, xtask delegates those
per-gate deadlines to GNU `timeout` when available so a timed-out cargo or test command
terminates as a process group, writes a receipt entry with `status: "timeout"`, and does
not continue running until the outer job timeout cancels the runner.

`merge-gate-shards` runs bounded groups of required gates with per-shard receipts and a
20-minute GitHub job timeout. The gate runner writes a receipt and log for each shard,
truncates very large gate logs before summarizing them, and records per-gate timeout
status from `.ci/gate-policy.yaml`. The `merge-gate` aggregate job reports those shard
results as the required merge-blocking status instead of hiding failures behind one
monolithic runner timeout.

---

## Section 4 — Required vs Informative Gates

The distinction is formal: it is encoded in `.ci/gate-policy.yaml` as `required: true` or
`required: false`.

**Required gates:**
- Failure blocks merge via the `ci/merge-gate` commit status check.
- The `merge-gate` GitHub Actions job collects all gate results in a receipt, then sets the
  commit status to `failure` if any required gate failed.
- No label manipulation can override a required gate failure. Live CI is authoritative.

**Informative (advisory) gates:**
- Failure is recorded in the receipt and the GitHub Step Summary.
- The receipt `summary.blocking_failures[]` will not include informative gate names.
- The PR can still merge if all required gates pass.
- Informative gate failures should be triaged and addressed in follow-up PRs or tracked in
  `debt-ledger.yaml`.

**Quarantine mechanism**: A gate can be temporarily demoted to advisory status via
`quarantine: true` in the policy. Quarantines are tracked in `.ci/debt-ledger.yaml` with
an expiration date. The `just debt-check` gate enforces that total quarantined-item counts
do not exceed the budget, and that no quarantine is older than its `quarantine_duration_days`
limit. This prevents the "quarantine and forget" failure mode.

---

## Section 5 — CI Receipts

Every gate run (local or CI) emits a structured receipt to `target/receipts/receipt.json`.
The receipt schema is formally defined in `.ci/receipt.schema.json`.

### What a Receipt Contains

```
receipt.json
├── schema_version       # "1.0.0"
├── metadata
│   ├── timestamp        # ISO 8601
│   ├── git_sha          # Full commit SHA (40 hex chars)
│   ├── git_branch
│   ├── toolchain        # rustc version, channel
│   ├── platform         # os, arch, is_wsl
│   └── trigger          # "ci-pr", "ci-merge", "manual", etc.
├── gates[]
│   ├── gate_name        # e.g. "unit_full"
│   ├── tier             # "pr_fast", "merge_gate", "nightly", "release"
│   ├── status           # "pass", "fail", "skip", "timeout", "error"
│   ├── required         # true/false (blocks merge if true and status is fail)
│   ├── duration_ms
│   ├── command          # exact command that ran
│   ├── exit_code
│   ├── output_summary   # truncated last N lines of output
│   └── metrics          # tests_total, tests_passed, warnings_count, etc.
├── summary
│   ├── total_gates
│   ├── passed / failed / skipped
│   ├── overall_status   # "pass", "fail", "partial"
│   └── blocking_failures[]  # names of required gates that failed
└── agent_receipt        # agent-oriented context for pr-responder automation
    ├── sha              # current HEAD SHA this receipt applies to
    ├── scope            # direct_crates, reverse_deps, risk_tags
    ├── selected_lanes   # lanes chosen by ci-scope
    ├── failures[]       # lane, summary, repro command
    └── suggested_next_actions[]
```

### How Agents Use Receipts

The `agent_receipt` sub-object is specifically designed for the `pr-responder` agent.
When CI fails, the responder reads:
- `agent_receipt.failures[].repro` — the exact command to reproduce the failure locally
- `agent_receipt.failures[].summary` — what went wrong
- `agent_receipt.suggested_next_actions[]` — suggested repair steps

The receipt is uploaded as a GitHub Actions artifact (`gate-receipt-<sha>`) with 7-day
retention, allowing post-hoc investigation of failures.

### SHA Binding and Staleness

A receipt is valid only for the SHA recorded in `metadata.git_sha`. When a new commit
is pushed to a branch:
- The old receipt becomes stale
- The `ci-green` label (if applied against the old SHA) becomes informational only
- The reconciler detects the mismatch and strips stale `ci-green` labels

This is the core principle from [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md):
the receipt records what was true at a specific SHA. Live `statusCheckRollup` tells you
what is true now.

---

## Section 6 — Live CI vs Label CI

This section summarizes the principle from [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md)
as it specifically applies to CI gates.

### The Key Distinction

| Signal type | What it means | When to use |
|-------------|--------------|-------------|
| Live `statusCheckRollup` for current HEAD SHA | CI is actually green/red right now | Merge-readiness decisions |
| `ci-green` label | "The green-ci agent ran a pass and it was green at that time" | Audit trail, ordering |
| `needs-ci-fix` label | "The green-ci agent flagged a problem at time of check" | Audit trail, routing |

**The `ci-green` label does not mean CI is green.** It means an agent ran a verification
pass. If a new commit has been pushed since the label was applied, the label is stale.

**Merge-readiness queries must use live CI:**
```bash
# Required: live statusCheckRollup for current HEAD SHA
gh pr view <N> --json statusCheckRollup \
  --jq '[.statusCheckRollup | group_by(.name) | .[] | sort_by(.completedAt) | last]'

# Required: live mergeStateStatus
gh pr view <N> --json mergeStateStatus
```

**Do not use `rtk gh pr checks` for merge-readiness decisions.** The `rtk` filter
summarizes individual job conclusions and can drop aggregator failures. PR #7016 showed
this: `rtk` reported "Passed: 14, Failed: 0" while `CI Gate (Merge-Blocking)` was
`FAILURE` on the latest SHA. Use raw `statusCheckRollup` queries for merge gates.
See [FAILURE_MODES.md — rtk gh pr checks Masks Aggregator Failure](FAILURE_MODES.md).

### Reconciler Behavior

The reconciler (planned at `xtask/src/tasks/queue_reconciler.rs`, tracked in #7085) continuously:
1. Queries live CI for each PR's current HEAD SHA
2. Compares live CI to `ci-green`/`needs-ci-fix` labels, stripping stale ones
3. For no-live-signal labels (all signoff labels), applies timeline precedence: later-applied wins when contradictions exist
4. Outputs derived queue state (which PRs are merge-ready, which are routing-blocked)

---

## Section 7 — Cancellation Cascades

### The Problem

GitHub Actions cancels in-progress runs when a new push arrives on the same branch (for PR
events). During rapid back-to-back master merges, every merge cancels the previous run.
The net result: no single master SHA may have a complete CI result.

This is the Cancellation Cascade from [FAILURE_MODES.md](FAILURE_MODES.md). Detection:
```bash
gh run list --branch master
# Shows multiple CANCELLED runs in rapid succession
```

### Current Mitigation

The `concurrency` block in `.github/workflows/ci.yml` is set to cancel-in-progress **only
for PR events** (`cancel-in-progress: ${{ github.event_name == 'pull_request' }}`). Master
push events and `merge_group` events are allowed to run to completion:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

This means: rapid pushes to the same PR branch cancel earlier runs (saving cost and
avoiding redundant CI on superseded SHAs), but pushes to master do not cancel each other.

The **batch-of-3 merge protocol** provides additional mitigation: merge at most 3 PRs per
batch, wait for master CI to complete between batches. This prevents the cancellation
cascade during high-throughput merge phases. See CLAUDE.md: "Merge in batches of 3 (CI
cancellation cascade)."

### SHA Staleness Check

The `preflight-latest-check` job guards against running CI on superseded PR SHAs:
```yaml
preflight-latest-check:
  # Compares the current SHA against the latest on the branch
  # If superseded, all downstream jobs are skipped (is_latest=false)
```

This prevents the CI system from wasting resources on older pushes that are already
superseded by a newer push to the same branch.

---

## Section 8 — The `merge_group` Trigger Gap

### Current State

GitHub Actions supports a `merge_group` event that fires when a PR enters the merge queue.
The `on:` block in `.github/workflows/ci.yml` already declares `merge_group: {}`, which
means the CI workflow will run on merge queue entries if a merge queue is enabled.

However, **no merge queue is currently configured** for this repository. The `merge_group`
trigger is declared in the workflow but is effectively dormant.

This is tracked as a future work item in #7072.

### Why This Matters

Without a merge queue:
- Frontdoor Proof runs on the PR branch (may be ahead of master)
- After merge, master CI validates the landed commit
- A brief window exists where two PRs can be "CI green" independently but conflict when
  both land on master

With a merge queue:
- GitHub rebases each PR onto the current merge queue head before running CI
- CI validates the PR-as-it-would-land, not just the PR-as-it-is
- Eliminates the window where two independent "green" PRs produce a broken master

### Future Work

When a merge queue is added (#7072), frontdoor and survivor-level checks should declare
`merge_group` in their `on:` triggers. The existing declaration in `ci.yml` is already
correct; no workflow change is needed — just the repository configuration to enable the
queue.

---

## Section 9 — Windows Guardrails

Windows-specific lanes run as a separate job (`windows-guardrails`) on `windows-latest`
runners. These are currently not gated by `draft-pr-check` (they run on all pushes where
`preflight-latest-check` passes), because Windows regressions have historically been
introduced by PRs whose Linux CI was clean.

**Current lanes:**

| Lane | Command | Catches |
|------|---------|---------|
| `compile` | `cargo check --locked --all-targets -p perl-module -p perl-lsp-rs -p perl-workspace` | Windows compile failures in key crates |
| `module-separator-regressions` | `cargo test -p perl-module --test path_comprehensive_unit_tests ...` | Path separator (backslash vs forward slash) regressions |
| `sandbox-fail-closed` | `cargo test -p perl-lsp-rs security::sandbox::tests::test_windows_sandbox_fails_closed` | Security sandbox behaves correctly on Windows |

**Known failure pattern** (see `feedback_xtask_fmt_false_cascade.md` in MEMORY.md): The
`Compile + PR Smoke + Windows Guardrails (module-separator-regressions)` triple-failure
pattern is the fingerprint of `xtask fmt` aborting at first failure. This looks like a
master cascade but is often N independent PR-side format issues. Verify on master before
declaring cascade.

---

## Section 10 — Local CI Tiers

CI runs locally via `just` commands. These match the remote tiers:

| Local command | Equivalent tier | When to run |
|---------------|----------------|-------------|
| `just pr-fast` | `pr_fast` gates | Before every push; installed as pre-push hook |
| `just ci-gate` (or `nix develop -c just ci-gate`) | `merge_gate` gates | Before merge, or to verify master health |
| `just ci-full` | `nightly` + `merge_gate` | Benchmarks, mutation, fuzzing |
| `cargo xtask gates --tier pr-fast` | `pr_fast` only | Targeted fast check |
| `cargo xtask gates --tier merge-gate` | `merge_gate` only | Targeted merge check |

Gate receipts are written to `target/receipts/receipt.json` in all cases. The receipt
from a local run and the receipt from a CI run use the same schema (`.ci/receipt.schema.json`),
differing only in `metadata.environment.type` ("local" vs "ci").

---

## See Also

- [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md) — umbrella system design, vocabulary
- [FAILURE_MODES.md](FAILURE_MODES.md) — operational failure patterns (Master Bit-Rot
  Cascade, xtask fmt False Cascade, Master Test Panic Blocker, CI Cancellation Cascade,
  Workflow PR-Only Trigger Observability Gap, rtk Masks Aggregator Failure)
- [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) — live CI vs `ci-green` label;
  reconciler behavior; merge-readiness query patterns
- [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — design philosophy behind the
  tiered approach
- [PIPELINE_GATES.md](PIPELINE_GATES.md) — the 7-gate PR pipeline model; where CI gates
  fit in Gate 5 (CI green)
- `.ci/gate-policy.yaml` — authoritative gate definitions, tiers, enforcement levels
- `.ci/receipt.schema.json` — receipt JSON schema
- `.github/workflows/ci.yml` — GitHub Actions workflow implementing Frontdoor Proof
- `xtask/src/tasks/ci_scope.rs` — diff classifier and lane selector
- `xtask/src/tasks/gates.rs` — gate runner with receipt emission
- `xtask/src/tasks/queue_reconciler.rs` — reconciler that grounds `ci-green` in live state (planned; tracked in #7085)
- Issue #7072 — merge queue implementation (enables `merge_group` trigger)

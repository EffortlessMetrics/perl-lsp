# Receipt Schema Reference

**Receipts are SHA-bound proof artifacts.** Every meaningful pass in the pipeline produces a
durable record binding its conclusion to the exact commit it evaluated. Future agents read prior
receipts as context — a comment trail over overwrite, not an overwrite. Receipts are the memory
of the conveyor.

Cross-references:
- `LIVE_SIGNALS_VS_LABELS.md` (planned, #7102) — live label truth vs receipt truth
- `RECONCILER_DEBUGGING.md` (planned, #7102) — how the reconciler reads receipts
- `LEARNING_CAPTURE_FORMAT.md` (planned, #7103) — how wisdom agents emit learning receipts
- `.ci/receipts/registry.toml` — registry of all known receipt types
- `.ci/receipt.schema.json` — JSON Schema for the gate-runner receipt format (full)

---

## 1. The Receipt Principle

A receipt answers: *"On commit SHA X, agent Y evaluated this PR and concluded Z."*

Three properties are non-negotiable:

1. **SHA-bound** — every receipt carries the `head_sha` it evaluated. If the PR advances,
   prior receipts are stale (still readable for history; no longer authoritative).
2. **Durable** — receipts are never overwritten. A new pass appends a new receipt. The most
   recent receipt per stage wins for routing decisions.
3. **Structured** — receipts carry a machine-readable JSON block so the reconciler can parse
   them without natural-language extraction.

Receipt freshness is checked by comparing the stored `head_sha` against the current PR HEAD.
This is distinct from label freshness; the `label-receipt-validate` skill codifies this check.

---

## 2. Receipt Forms

Receipts appear in two physical forms:

| Form | Location | Producer |
|------|----------|----------|
| **JSON file** | `target/receipts/<stage>.json` | Gate runner (`cargo xtask gates`), xtask tasks |
| **PR/issue comment** | GitHub PR or issue comment | Review agents, reconciler, skill steps |

Both forms carry the same logical content. JSON files are committed to CI artifacts.
Comment receipts are the primary input for the reconciler and subsequent agents — they
persist on the PR even when artifacts expire.

---

## 3. Common Envelope

Every receipt, regardless of type, carries these fields. Fields marked **required** must be
present for the reconciler to process the receipt.

```json
{
  "check": "<stage-id>",
  "schema_version": 1,
  "event": "<pull_request|merge_group|push|local>",
  "pr": 1234,
  "head_sha": "<40-char hex>",
  "base_sha": "<40-char hex>",
  "verdict": "<pass|fail|warn|skipped>",
  "created_at": "2026-04-27T12:00:00Z"
}
```

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `check` | string | yes | Stage identifier (e.g. `"merge-readiness"`, `"review"`) |
| `schema_version` | integer | yes | Schema major version; current is `1` |
| `event` | string | yes | Trigger context: `pull_request`, `merge_group`, `push`, `local` |
| `pr` | integer | yes (for PR receipts) | PR number; omit or null for issue-only receipts |
| `head_sha` | string | yes | Full 40-char SHA this receipt was evaluated against |
| `base_sha` | string | no | Merge-base SHA (required for merge-readiness and diff receipts) |
| `verdict` | string | yes | See §4 per-stage values — always a controlled vocabulary |
| `created_at` | string | no | ISO 8601 timestamp; omit only for legacy receipts |

> **Note on `actor`:** The common envelope does not define a mandatory `actor` field today.
> Stage-specific receipts use `producer` (review) or rely on the GitHub comment author.
> A future `schema_version: 2` will standardize `actor` across all receipt types.

---

## 4. Stage-Specific Receipt Types

The following sections document each receipt type: its required fields, controlled vocabulary
for `verdict`, and where it is emitted.

### 4.1 Gate Runner (`check: "<gate-name>"`)

**Emitter:** `cargo xtask gates --receipt` → `target/receipts/receipt.json`
**Schema file:** `.ci/receipt.schema.json`
**Registry entry:** each gate appears under its own `check` name

The gate runner receipt is the most detailed. It contains a `gates` array (one entry per
gate executed) plus a `summary` and optional `agent_receipt` block.

```json
{
  "schema_version": "1.0.0",
  "metadata": {
    "timestamp": "2026-04-27T12:00:00Z",
    "git_sha": "<40-char hex>",
    "git_branch": "impl/7083-receipt-schema",
    "git_dirty": false,
    "toolchain": { "rustc_version": "rustc 1.89.0 ..." },
    "platform": { "os": "linux", "arch": "x86_64" },
    "environment": { "type": "local" }
  },
  "gates": [
    {
      "gate_name": "fmt",
      "tier": "pr_fast",
      "status": "pass",
      "required": true,
      "duration_ms": 2340,
      "command": "cargo fmt --check --all",
      "exit_code": 0,
      "metrics": { "files_checked": 156 }
    }
  ],
  "summary": {
    "total_gates": 12,
    "passed": 12,
    "failed": 0,
    "skipped": 0,
    "total_duration_ms": 87432,
    "overall_status": "pass"
  }
}
```

Key fields:
- `schema_version` is a **semver string** (`"1.0.0"`) in this receipt — inconsistent with
  other receipts that use an integer. See §7 (Inconsistencies).
- `summary.overall_status`: `"pass" | "fail" | "partial"`
- `gates[*].status`: `"pass" | "fail" | "skip" | "timeout" | "error"`
- `gates[*].tier`: `"pr_fast" | "merge_gate" | "nightly" | "release"`

### 4.2 Merge Readiness (`check: "merge-readiness"`)

**Emitter:** `cargo xtask merge-ready emit --pr <N>` → `target/receipts/merge-readiness.json`
**Schema file:** `.ci/receipts/schemas/merge-readiness.schema.json`
**Rust struct:** `MergeReadinessReceipt` in `xtask/src/tasks/merge_ready.rs`

```json
{
  "check": "merge-readiness",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 1234,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "base_sha": "1234567890abcdef1234567890abcdef12345678",
  "gate_graph_version": "fnv1a64:a3f8b2c1d9e04567",
  "required_checks": ["fmt", "clippy", "unit_tests", "lsp_smoke"],
  "review_evidence": ["reviewed-deep", "ci-green"],
  "blocker_labels_absent": true,
  "verdict": "valid",
  "expires_when": "on_new_commit_or_base_or_policy_change"
}
```

`verdict` vocabulary:

| Value | Meaning |
|-------|---------|
| `valid` | Receipt is fresh and all conditions satisfied |
| `stale_head` | PR has new commits since receipt was emitted |
| `stale_base` | Master has advanced since receipt was emitted |
| `stale_gate_graph` | Gate policy changed since receipt was emitted |
| `blocked` | `blocker_labels_absent = false` or `verdict = "blocked"` |
| `missing` | No receipt file found at expected path |

`gate_graph_version` is an FNV1a-64 hash over all gate policy files and required checks.
Any policy change invalidates existing receipts automatically.

### 4.3 Failure Classifier (`check: "failure-classifier"`)

**Schema file:** `.ci/receipts/schemas/failure-classifier.schema.json`
**Extends:** common-gate-receipt

```json
{
  "check": "failure-classifier",
  "schema_version": "1",
  "event": "pull_request",
  "verdict": "fail",
  "classification": "stale_base"
}
```

`classification` vocabulary: `code_regression | infra_failure | stale_base | master_red | skipped | unknown`

### 4.4 Review (`check: "review"`)

**Schema file:** `.ci/receipts/schemas/review.schema.json`
**Emitter:** reviewer / reviewer-deep agents

The review receipt is evidence-only and carries substantive observations, not just a verdict.
It uses `kind` instead of `check` (a pre-unification inconsistency — see §7).

```json
{
  "kind": "review",
  "producer": "reviewer-deep",
  "pr": 1234,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "base_sha": "1234567890abcdef1234567890abcdef12345678",
  "verdict": "clean",
  "material_observations": [
    "All edge cases from spec covered",
    "No banned patterns (`unwrap`, `expect`) in production code"
  ],
  "negative_checks": [
    "No cross-PR contamination detected",
    "No audit-trail directory from another issue"
  ],
  "blockers": [],
  "next_routes": ["signoff_clean"],
  "supersedes": null
}
```

`verdict` vocabulary: `clean | needs_builder_fix | needs_diff_fix | needs_human | blocked_unknown`

Constraint: `verdict: "clean"` requires `next_routes` to contain `"signoff_clean"` and
`material_observations` must be non-empty (the schema enforces this via `allOf`).

### 4.4.1 Review label projection receipt (`kind: "review_receipt"`)

Reviewer agents should emit a structured review receipt and/or matching PR comment for the
**current PR head SHA**; the queue reconciler is the label authority and projects/repairs labels
from this evidence. Reviewers should not treat manual label edits as the primary control path.

```json
{
  "kind": "review_receipt",
  "schema_version": 1,
  "pr": 1234,
  "sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "approved",
  "review_depth": "deep",
  "fix_forward_applied": false,
  "blocking_findings": [],
  "labels_projected": []
}
```

`verdict` vocabulary: `approved | needs_builder | needs_diff | needs_ci`

`review_depth` vocabulary: `haiku_first | deep | security`

### 4.5 Aggregator (`check: "aggregator"`)

**Schema file:** `.ci/receipts/schemas/aggregator-receipt.schema.json`
**Emitter:** gate aggregation tasks

Rolls up multiple gate receipts into one control-plane summary.

```json
{
  "check": "aggregator",
  "schema_version": "1",
  "event": "pull_request",
  "verdict": "pass",
  "classification": "unknown",
  "subreceipts": [
    {
      "name": "fmt",
      "selected": true,
      "required": true,
      "verdict": "pass",
      "classification": "unknown",
      "repro": { "command": "cargo fmt --check --all" }
    }
  ],
  "missing_receipts": [],
  "repro": { "command": "cargo xtask gates" }
}
```

### 4.6 Queue Snapshot (`check: "queue-snapshot"`)

**Schema file:** `.ci/receipts/schemas/queue-snapshot.schema.json`
**Context:** Reconciler reads queue snapshots as its input

```json
{
  "snapshot_id": "snap-2026-04-27T12:00:00Z",
  "captured_at": "2026-04-27T12:00:00Z",
  "repository": "EffortlessMetrics/perl-lsp",
  "default_branch": "master",
  "master_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "prs": [],
  "buckets": {},
  "leases": []
}
```

Note: this receipt does not yet follow the common envelope (no `check`, `schema_version`,
`event`, `verdict` fields). See §7 (Inconsistencies).

### 4.7 Agent Receipt (`check: "agent-task"`)

**Schema file:** `.ci/receipts/schemas/agent-receipt.schema.json`
**Rust struct:** `AgentReceipt` in `xtask/src/tasks/agent_receipt.rs`

Agent receipts bind a mutation claim to a lease. They are validated before any write
operation is accepted.

```json
{
  "schema_version": 1,
  "task_id": "task-7083-abcd",
  "snapshot_id": "snap-2026-04-27T12:00:00Z",
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "lease_path": "target/leases/task-7083-abcd.json",
  "required_output_schema": ".ci/receipts/schemas/review.schema.json",
  "received_at": "2026-04-27T12:05:00Z",
  "idempotency_key": "7083-review-abcdef12",
  "mutation": "signoff_clean",
  "status": "accepted"
}
```

### 4.8 Reconciler Receipt (future, `check: "reconciler"`)

The queue reconciler (in flight via #7085) will emit `target/receipts/queue-reconcile.json`.
The proposed schema (not yet implemented) is:

```json
{
  "check": "reconciler",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 1234,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "pass",
  "created_at": "2026-04-27T12:00:00Z",
  "contradictions_resolved": [
    {
      "labels_before": ["ci-green", "needs-ci-fix"],
      "labels_after": ["needs-ci-fix"],
      "rationale": "needs-ci-fix takes precedence over stale ci-green"
    }
  ],
  "stripped_labels": ["ci-green"],
  "applied_labels": [],
  "evidence": [
    {
      "action": "strip ci-green",
      "source_receipt": "target/receipts/green-ci.json",
      "source_sha": "abcdef1234567890abcdef1234567890abcdef12",
      "reason": "HEAD advanced; ci-green is stale"
    }
  ]
}
```

### 4.9 Salvage Receipt (future, `check: "salvage"`)

Emitted by salvage classification agents when a stale or dirty PR is triaged.

```json
{
  "check": "salvage",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 1234,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "pass",
  "created_at": "2026-04-27T12:00:00Z",
  "salvage_verdict": "CHERRY_PICK",
  "superseder_pr": null,
  "lessons_harvested": [
    "Test coverage for edge case X extracted to issue #7199"
  ],
  "cost_estimate": "low",
  "rationale": "One topical commit; cherry-pick to fresh branch off master"
}
```

`salvage_verdict` vocabulary:
`SALVAGE_REBASE | CHERRY_PICK | EXTRACT_TESTS | EXTRACT_IMPL | CLOSE_SUPERSEDED | CLOSE_CONTAMINATED | CLOSE_PREMISE_OBSOLETE`

### 4.10 Curator Receipt (future, `check: "curator"`)

Emitted by ensemble-curator when resolving a Codex cluster.

```json
{
  "check": "curator",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 1234,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "pass",
  "created_at": "2026-04-27T12:00:00Z",
  "cluster_id": "codex-burst-2026-04-27",
  "winner_pr": 1234,
  "loser_prs": [1235, 1236],
  "harvested_tests": [
    { "from_pr": 1235, "test_name": "test_edge_case_empty_input" }
  ],
  "harvested_ideas": [
    { "from_pr": 1236, "description": "Consider using split_qualified_name here", "follow_up_issue": 7200 }
  ],
  "follow_up_issues": [7200]
}
```

---

## 5. Comment Receipts

When an agent posts its verdict as a PR or issue comment, the comment must follow this
structure so the reconciler can parse it.

### 5.1 Required Structure

```
## <Stage> <VerdictName>

<human-readable summary, 1-5 sentences>

### Observations

- <observation 1>
- <observation 2>

### Machine-readable

```json
{ <receipt JSON conforming to the relevant schema> }
```

---
*<stage-id> — <one-line description of this agent's role>.*
```

Examples of the required header format:
- `## Reconciler action` (reconciler)
- `## Diff-audit verdict` (diff-auditor)
- `## CI Verification` (green-ci)
- `## Review Receipt` (reviewer-deep)

### 5.2 Parsing Rules

The reconciler locates the machine-readable block by:
1. Finding the last `## <Stage>` header in the comment
2. Finding the first ` ```json` fence after that header
3. Parsing everything until the matching ` ``` ` close

If multiple receipts exist for the same stage on the same PR, the reconciler uses the
**most recent** (last in comment history) as authoritative.

### 5.3 Comment vs JSON File

JSON file receipts and comment receipts carry the same logical content but are used
differently:

| Dimension | JSON file | Comment |
|-----------|-----------|---------|
| Persistence | CI artifact TTL (typically 90 days) | Permanent on PR |
| Primary consumer | Tooling (`cargo xtask gate-receipts validate`) | Reconciler, subsequent agents |
| Overwrite policy | Written once per run | Never overwritten; append new comment |
| Discovery | Known path (`target/receipts/<stage>.json`) | Parsed from comment history |

---

## 6. Versioning

`schema_version` in all receipts tracks breaking changes.

| Change type | Version bump |
|-------------|--------------|
| New optional field added | No bump (backward compatible) |
| Existing field renamed or removed | Major bump |
| `verdict` vocabulary extended with new values | Minor bump (note in changelog) |
| `verdict` vocabulary narrowed (values removed) | Major bump |

**Current version:** `1` (integer) for all receipts except the gate-runner receipt, which
uses `"1.0.0"` (semver string) — a known inconsistency tracked for unification in v2.

Receipts older than the current major version are still valid for human reading and
historical audit. Automated tooling may decline to parse them; agents should surface a
warning rather than a hard failure when encountering an older receipt.

---

## 7. Known Inconsistencies

The following inconsistencies exist across current emitters. They are documented here for
transparency. A future `schema_version: 2` sweep will resolve them.

### 7.1 `schema_version` type mismatch

| Emitter | Field type | Value example |
|---------|-----------|---------------|
| `merge_ready.rs` (`MergeReadinessReceipt`) | `u32` / integer | `1` |
| `agent_receipt.rs` (`AgentReceipt`) | `u32` / integer | `1` |
| `gates.rs` (`Receipt`) / `receipt.schema.json` | string (semver) | `"1.0.0"` |
| `common-gate-receipt.schema.json` | string | `"1"` |

**Proposed unification:** integer for all receipts, with `schema_version: 2` being the
first version that enforces this uniformly.

### 7.2 `verdict` vocabulary fragmentation

Different receipt types use different controlled vocabularies for their `verdict` / outcome
field:

| Receipt type | Vocabulary |
|-------------|-----------|
| common-gate-receipt, aggregator | `pass | fail | warn | skipped` |
| merge-readiness | `valid | stale_head | stale_base | stale_gate_graph | blocked | missing` |
| review | `clean | needs_builder_fix | needs_diff_fix | needs_human | blocked_unknown` |
| gate-runner `summary.overall_status` | `pass | fail | partial` |

These are semantically distinct stages and the different vocabularies are intentional
(merge-readiness verdicts describe freshness, not outcome). However, the common envelope
(§3) defines `verdict` as from `pass|fail|warn|skipped`, which merge-readiness and review
receipts do not conform to. **Proposed resolution:** rename the outer envelope field to
`outcome` in v2, and reserve `verdict` for stage-specific interpretation.

### 7.3 `check` vs `kind` inconsistency

The review receipt schema uses `"kind": "review"` where all other receipts use
`"check": "<stage>"`. The `gate_receipts.rs` validator looks for the `check` field; the
review receipt would fail common-field validation. **Proposed resolution:** replace `kind`
with `check` in review receipt schema and emitters (v2 migration).

### 7.4 `AgentReceipt` name collision

There are two structs named `AgentReceipt` with unrelated shapes:
- `xtask/src/tasks/agent_receipt.rs` — mutation-claim receipt bound to a lease
- `xtask/src/tasks/gates.rs` `AgentReceipt` — CI scope/lane summary embedded in the gate runner receipt

These are distinct concepts. **Proposed resolution:** rename the gates.rs inner struct to
`GateAgentContext` in v2.

### 7.5 Queue snapshot missing common envelope

The queue-snapshot schema lacks `check`, `schema_version`, `event`, and `verdict` — the
four fields required by `gate_receipts.rs` common-field validation. It is currently
consumed only by the reconciler (which has its own parsing logic) so this gap has not
caused failures. **Proposed resolution:** wrap queue snapshots in the common envelope in v2.

---

## 8. Where Receipts Go

| Receipt type | JSON path | Comment |
|-------------|-----------|---------|
| Gate runner | `target/receipts/receipt.json` | — |
| Merge readiness | `target/receipts/merge-readiness.json` | — |
| Reconciler (planned) | `target/receipts/queue-reconcile.json` | `## Reconciler action` on PR |
| Review | — | `## Review Receipt` on PR |
| Diff audit | — | `## Diff-audit verdict` on PR |
| CI verification | — | `## CI Verification` on PR |
| Agent receipt | `target/receipts/agent-<task-id>.json` | — |
| Queue snapshot | `target/receipts/queue-snapshot-<id>.json` | — |

Receipts in `target/receipts/` are committed to CI artifacts and retained for the
configured retention period (see `.ci/gate-policy.yaml` `global.artifact_retention_days`).

Comment receipts persist indefinitely on the PR. Agents read prior receipts as context
before taking action — the most recent receipt per stage per SHA is authoritative for
routing decisions.

---

## 9. Worked Examples

### 9.1 Green-CI receipt for a clean PR

PR #7083, all checks green on the current HEAD:

```json
{
  "check": "green-ci",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 7083,
  "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "pass",
  "created_at": "2026-04-27T14:30:00Z",
  "head_sha_checks": [
    { "name": "Compile All Targets",    "status": "success", "sha": "abcdef12" },
    { "name": "PR Smoke",               "status": "success", "sha": "abcdef12" },
    { "name": "Clippy",                 "status": "success", "sha": "abcdef12" },
    { "name": "Windows Guardrails",     "status": "success", "sha": "abcdef12" }
  ],
  "classified_failures": [],
  "recommended_route": "proceed_to_diff_audit"
}
```

The corresponding comment on the PR:

```
## CI Verification

**HEAD SHA:** `abcdef12`
**Verdict:** GREEN

| Check | Status | SHA |
|-------|--------|-----|
| Compile All Targets | pass | abcdef12 |
| PR Smoke | pass | abcdef12 |
| Clippy | pass | abcdef12 |
| Windows Guardrails | pass | abcdef12 |

```json
{ "check": "green-ci", "schema_version": 1, "event": "pull_request",
  "pr": 7083, "head_sha": "abcdef1234567890abcdef1234567890abcdef12",
  "verdict": "pass", "created_at": "2026-04-27T14:30:00Z",
  "classified_failures": [], "recommended_route": "proceed_to_diff_audit" }
```

---
*green-ci — SHA-verified CI freshness check.*
```

### 9.2 Reconciler receipt resolving a label contradiction

PR has both `ci-green` (from an earlier SHA) and `needs-ci-fix` (from green-tdd finding a
bug). The reconciler strips `ci-green` because `needs-ci-fix` is a routing label that
takes precedence, and because `ci-green` is stale relative to the current HEAD.

```json
{
  "check": "reconciler",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 6900,
  "head_sha": "fedcba0987654321fedcba0987654321fedcba09",
  "verdict": "pass",
  "created_at": "2026-04-27T15:00:00Z",
  "contradictions_resolved": [
    {
      "labels_before": ["ci-green", "needs-ci-fix"],
      "labels_after":  ["needs-ci-fix"],
      "rationale": "`ci-green` is stale (emitted for SHA abcdef12, current is fedcba09); `needs-ci-fix` routing label takes precedence — must be resolved before merge"
    }
  ],
  "stripped_labels": ["ci-green"],
  "applied_labels":  [],
  "evidence": [
    {
      "action": "strip ci-green",
      "source": "target/receipts/merge-readiness.json",
      "source_sha": "abcdef1234567890abcdef1234567890abcdef12",
      "reason": "HEAD advanced to fedcba09; stale receipt is not authoritative"
    }
  ]
}
```

### 9.3 Diff-audit receipt with SCOPE_DRIFT verdict

PR #6850 adds files outside the scope claimed in its title.

```json
{
  "check": "diff-audit",
  "schema_version": 1,
  "event": "pull_request",
  "pr": 6850,
  "head_sha": "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
  "verdict": "fail",
  "created_at": "2026-04-27T16:00:00Z",
  "scope_match": "DRIFT",
  "audit_trail_check": "PASS",
  "files_changed": 23,
  "scope_drift_findings": [
    {
      "file": "crates/perl-dap/src/eval/validator.rs",
      "lines": "+89",
      "reason": "Assignment operator precedence change unrelated to PR scope (perl-parser symbol resolution)"
    }
  ],
  "verdict_label": "needs-diff-fix",
  "recommended_action": "strip the unrelated change in perl-dap/src/eval/validator.rs before re-review"
}
```

The corresponding comment:

```
## Diff-audit verdict

**Files changed:** 23 (+412 / -87)
**Commits:** 3

### Spec alignment: PARTIAL
Acceptance criteria for symbol resolution: covered.
One file outside stated scope found.

### Cleanliness: ARTIFACTS FOUND
`crates/perl-dap/src/eval/validator.rs:89` — assignment operator precedence
change with no issue reference; not mentioned in PR title, body, or spec.

### Verdict: SCOPE DRIFT

Unrelated change bundled in diff. Strip the perl-dap validator change before merge.

---
*Diff auditor — final coherence check before merge.*
```

---

## 10. Registry

All receipt types must be registered in `.ci/receipts/registry.toml`. The validator
(`cargo xtask gate-receipts validate`) uses this registry to:
1. Confirm the receipt's `check` field is known
2. Locate the per-type JSON Schema for required-field validation
3. Report unknown checks as errors rather than silently ignoring them

To register a new receipt type, add an entry:

```toml
[[receipt]]
check = "my-new-stage"
schema = ".ci/receipts/schemas/my-new-stage.schema.json"
description = "One-line description of what this stage proves."
producer = "cargo xtask my-new-stage"
required_fields = ["head_sha", "verdict"]
```

The four common required fields (`check`, `schema_version`, `event`, `verdict`) are always
validated by the common-field validator regardless of the per-type schema.

# Codex Agent Implementation Guide (Control-Plane Modernization)

This guide defines how implementation agents contribute to the receipt-driven CI/control-plane model tracked by **#6853**.

## Codex agent guardrails

1. Inspect current `master` first so already-merged work is not reimplemented.
2. If the target is already implemented, deliver a minimal follow-up (or explicitly report a no-op).
3. Keep PRs narrowly scoped to one concern.
4. Do not edit unrelated high-churn global files.
5. Do not claim branch/ruleset enforcement was changed unless it was actually changed.

## Operating model for agents

- Agents do not own mutable canonical state.
- Agents emit receipts as evidence.
- Routing-critical operations must emit receipts.
- Reconciler/state builder derives canonical state.
- Labels are projected UI, not authority.

## Receipt locations

- Generated runtime receipts: `target/receipts/*.json`
- Committed schemas: `.ci/schemas/*.schema.{json,yaml}`
- Registry: `.ci/GATE_REGISTRY.toml`

When adding new receipt types, update schema + registry in the same scoped PR.

## Workflow rules agents must preserve

- Required-style workflows must always run and no-op internally when not applicable.
- Do not path-filter required-style workflows.
- For modernization work, ensure workflows are wired for:
  - `pull_request`
  - `merge_group`
  - `push` to `master`
- Use event-aware concurrency groups so events do not cancel unrelated truth-building runs.
- Use final aggregators to publish a single canonical pass/fail signal for the control-plane view.

## Staged rollout expectations

- **P0**: impossible states impossible.
- **P1**: receipts -> state -> labels.
- **P2**: Parser Ratchet scoped gate.
- **P3**: leases/worktree/queue health.
- **P4**: release evidence and scenario gates.

Agent PRs should state which stage they advance and what invariant/evidence boundary they add.

## Partial-closeout hygiene

Use close keywords intentionally:

- For scaffold/partial work: `Refs #6853` or `Part of #6853`.
- Use `Closes` / `Fixes` / `Resolves` only when acceptance criteria for the target issue are complete.

This avoids premature issue closure during staged rollout.

## Hard compatibility: CI-efficiency invariants (EffortlessMetrics)

When proposing CI-efficiency changes, preserve these invariants. "Cheaper" that regresses routing truth or queue semantics is a reject.

### 1) Concurrency semantics for heavy/core workflows

- Do **not** set `cancel-in-progress: true` on heavy/core Rust PR workflows unless the repository explicitly documents that cancellation is safe.
- Required model for heavy/core lanes:
  - one active run continues;
  - one pending replacement slot is allowed;
  - newest queued run replaces older pending runs;
  - active run is not killed near completion.
- Preferred pattern:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: false
```

### 2) Change classification must be truthful

- Do not treat all file changes as Rust source changes.
- Metadata/control-plane-only shapes should route to light paths unless mixed with real Rust/build/test edits.
- Typical light/control-plane surfaces include:
  - `docs/**`, markdown-only changes, `README*`, `CHANGELOG*`, `SECURITY*`, `CONTRIBUTING*`
  - `policy/**`, `plans/**`, `badges/**`, `AGENTS.md`
  - `.github/CODEOWNERS`, `.github/dependabot.yml`, PR templates
  - `.codex/campaigns/**`, `docs/tracking/**`, `ci/hardware/**` receipts
  - `.rails/**`, `.uselesskey/**`
- Workflow edits are special:
  - `.github/workflows/**` must not be routed as docs-light.
  - Route workflow-only changes to minimal hosted workflow validation/safety.

### 3) Default PR policy

Classify first, then pick the cheapest truthful lane:

- docs/control-plane-only -> no Rust compile.
- workflow-only -> hosted YAML/workflow validation, no full Rust lane.
- Rust/build/test touched -> routed `rust-small` (self-hosted where configured).
- hardware/GPU/receipt-only -> syntax/receipt validation only.
- unknown or mixed -> `rust-small` (not full CI).
- full CI requires explicit trigger (label/manual dispatch/main push/release/schedule/merge queue).

### 4) Hosted fallback policy

- Do not silently replace a self-hosted `rust-small` path with a hosted full Rust equivalent.
- Fork PRs may use a tiny hosted safe lane.
- Missing runner readiness/idle capacity/token errors must not auto-trigger 75-120 minute hosted fallbacks.
- Expensive hosted fallbacks must be explicit (labels/inputs such as `full-ci`, `allow-github-hosted`, `ci-budget-ack`).

### 5) Artifact policy

- Do not upload bulky artifacts on every default PR run.
- Prefer upload-on-failure with short retention (for example 3-7 days).
- If receipts are policy-required, keep them small and avoid always-upload paths on docs/control-plane-only lanes.

### 6) Required validation for CI-only efficiency PRs

Every CI-efficiency PR must include:

1. `git diff --check`
2. YAML parse check for each edited workflow
3. Classification dry-run/unit checks that cover at least:
   - docs-only
   - `.rails/**`
   - `.uselesskey/**`
   - workflow file change
   - Rust file change
   - mixed docs + Rust
4. Explicit confirmation that heavy/core no-cancel semantics were preserved unless intentionally changed and documented.

### Review reject checklist (fast gate)

Reject CI-efficiency PRs that cannot clearly answer "yes" to all:

1. Heavy/core workflows still preserve `cancel-in-progress: false` semantics.
2. Metadata/control-plane-only changes avoid Rust CI.
3. Workflow edits are not routed through docs-light.
4. Hosted fallback remains tiny or explicitly budget-acknowledged.
5. The change reduces real billable runner work instead of shifting cost.

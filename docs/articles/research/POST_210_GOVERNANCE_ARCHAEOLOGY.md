# Post-`#210` Governance Archaeology

## Question

What actually landed after issue `#210` asked for merge-blocking gates, receipts,
and check-run lifecycle handling?

## Short Answer

`#210` did not just produce one gate file or one status command. It seeded a
whole trust stack:

- the canonical gate policy moved into `.ci/gate-policy.yaml`
- gate execution gained a structured receipt layer in `.ci/receipt.schema.json`
- `xtask` grew first-class `gates` and `update-status` tasks
- `just` exposed those paths as `status-update`, `status-check`, and gate
  recipes
- the forensics/casebook layer later audited the numbers, claims, and receipts

What did not disappear was the debt `#210` pointed at. The repo still has
explicit blockers, lagging surfaces, and later audit prompts that treat proof
governance as an ongoing maintenance problem rather than a one-time fix.

## 1. `#210` Becomes A Planning Anchor

The clearest early committed traces are in project planning docs:

- [`docs/project/MILESTONES.md`](../../../docs/project/MILESTONES.md) lists
  `#210` as "Merge gates formalization" and explicitly says `#211` blocks it.
- [`docs/project/ORIENTATION.md`](../../../docs/project/ORIENTATION.md) repeats
  the same dependency: merge gates come after CI pipeline cleanup.
- [`docs/forensics/IMPLEMENTATION_PHASES.md`](../../../docs/forensics/IMPLEMENTATION_PHASES.md)
  puts `#210` in Phase A, "Trust Surface Stabilization", and says it defines
  what passing means for everything else.

That is important because it shows `#210` was treated as a root contract, not a
single implementation ticket.

## 2. The Gate Policy Lands In `.ci`

The policy side of the lineage is now explicit in committed config:

- [`.ci/gate-policy.yaml`](../../../.ci/gate-policy.yaml) defines the gate
  tiers, required gates, triggers, timeouts, retries, and budgets.
- [`.ci/receipt.schema.json`](../../../.ci/receipt.schema.json) defines the
  machine-readable receipt contract for gate runs, including metadata, per-gate
  results, summaries, and diffs.
- [`.ci/benchmark-thresholds.yaml`](../../../.ci/benchmark-thresholds.yaml)
  shows the same trust surface widening into performance alerts, while still
  marking performance gating as blocked on `#211/#210`.
- [`.ci/debt-ledger.yaml`](../../../.ci/debt-ledger.yaml) keeps `#210` in the
  debt language as a prior example of quarantines and gate-linked cleanup.

The key mutation is that proof is no longer just narrative. It is typed policy.

## 3. `xtask` Turns Policy Into Runtime

The runtime counterpart is in `xtask`:

- [`xtask/src/tasks/gates.rs`](../../../xtask/src/tasks/gates.rs) reads
  `.ci/gate-policy.yaml`, runs the gates, and emits receipts.
- [`xtask/src/tasks/update_status.rs`](../../../xtask/src/tasks/update_status.rs)
  recomputes the evidence-backed status docs from source data and exits
  non-zero if they drift.
- [`xtask/src/main.rs`](../../../xtask/src/main.rs) exposes both as first-class
  subcommands.
- [`scripts/run-gates.sh`](../../../scripts/run-gates.sh) preserves the older
  shell path, but the comment trail makes clear it is a compatibility runner,
  not the long-term center of gravity.

This is the main engineering answer to `#210`: gates and status are now
mechanized, not hand-maintained prose.

## 4. `just` Makes The Surfaces Usable

The operator-facing layer is the `just` recipes:

- [`justfile`](../../../justfile) maps `status-update` and `status-check` to
  `xtask update-status`.
- The same file exposes `gates`, `gates-json`, and `gates-legacy`, with the
  modern path routed through `cargo xtask gates --receipt`.
- `ci-gate` remains the canonical pre-push merge gate, so the repo keeps a
  simple local command even as the underlying machinery becomes more structured.

That matters historically because it shows the governance layer did not stay in
docs. It became a one-command operator surface.

## 5. The Status Layer Was Externalized, Not Invented

`#210` also pushed the repo toward a clean separation between evidence and
planning:

- [`docs/project/CURRENT_STATUS.md`](../../../docs/project/CURRENT_STATUS.md) is
  the evidence document.
- [`docs/project/ROADMAP.md`](../../../docs/project/ROADMAP.md) is the planning
  document.
- `xtask update-status` regenerates the computed blocks in both.
- `just status-check` fails when those blocks drift from the computed values.

That is the practical result of the `#210` lineage: current status stopped being
an opinionated summary and became a verifiable projection of source state.

## 6. Forensics And Casebook Became The Audit Layer

The later layer is where the repo audits its own truth surface:

- [`docs/forensics/README.md`](../../../docs/forensics/README.md) defines the
  forensics workflow: issue work orders, post-PR dossiers, and swarm
  coordination.
- [`docs/forensics/WORK_ORDER_FORMAT.md`](../../../docs/forensics/WORK_ORDER_FORMAT.md)
  turns analysis into contract blocks with exit criteria and measurement
  contracts.
- [`docs/forensics/prompts/measurement-auditor.md`](../../../docs/forensics/prompts/measurement-auditor.md)
  audits whether reported numbers actually match receipts and commands.
- [`docs/forensics/INDEX.md`](../../../docs/forensics/INDEX.md) and
  [`docs/project/CASEBOOK.md`](../../../docs/project/CASEBOOK.md) preserve the
  best examples as recoverable exhibits.
- [`docs/project/METRICS_PROVENANCE.md`](../../../docs/project/METRICS_PROVENANCE.md)
  formalizes the provenance fields needed for honest metrics.

This is the strongest proof that `#210` was not only about gates. It helped
create an audit culture around gates.

## 7. What Stayed As Recurring Debt

The repo never pretends the trust stack is finished:

- `#211` remains a named blocker in the milestone docs.
- performance gating is still marked as blocked in `.ci/benchmark-thresholds.yaml`.
- the debt ledger keeps quarantine, resolution, and expiry semantics alive.
- the forensics prompts explicitly allow auditors to reject unstable or
  dishonest comparisons.

So the post-`#210` story is not "problem solved". It is "the repo learned how
to keep the problem visible and governable."

## 8. Strongest Evidence-Backed Claims

1. Issue `#210` is a root governance request, not a narrow implementation ticket.
2. The concrete descendants are `.ci/gate-policy.yaml`, `.ci/receipt.schema.json`,
   `xtask gates`, `xtask update-status`, and the `just status-*` surfaces.
3. The `docs/forensics/` and `docs/project/CASEBOOK.md` layers are later audit
   surfaces that inspect the same trust system.
4. `#211` and the benchmark-threshold docs show the gate story stayed partially
   blocked and therefore remained live debt.
5. The repo's current operating model is the result of externalizing proof,
   not just documenting it.

## See Also

- [RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md](RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md)
- [VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md](VALIDATOR_BLIND_SPOT_ARCHAEOLOGY.md)
- [TRUTH_SURFACE_ARCHAEOLOGY.md](TRUTH_SURFACE_ARCHAEOLOGY.md)
- [CASEBOOK_FORENSICS_ARCHAEOLOGY.md](CASEBOOK_FORENSICS_ARCHAEOLOGY.md)

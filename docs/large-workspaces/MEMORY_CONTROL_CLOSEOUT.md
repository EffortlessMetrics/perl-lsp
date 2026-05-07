# Memory Control Closeout

This page closes the retained-state memory incident as an operating lane. The
fixes, counters, receipts, trend renderer, triage template, focused regression
tests, and retained-state policy guard are now in place. New memory work should
start from a failing receipt, a new retained owner, or an explicit subsystem
change, not from a broad leak hunt.

## Evidence Map

| Surface | Evidence |
|---------|----------|
| Original retained-state leak | #8064 plugged session-creep paths across LSP caches, stream sessions, workspace index cleanup, and lookup miss behavior. |
| Plateau guardrails | #8072 added LSP churn harnesses, plateau checks, lifecycle helpers, stale background-index guards, and PR/nightly memory workflows. #8074 registered structured memory plateau receipts. |
| Runtime pressure counters | #8088 added `RuntimePressureSnapshot` counters for async pressure surfaces and extended retained-state inventory policy coverage. |
| Diagnostics churn | #8115 added retained-state coverage for diagnostics churn after open/change/diagnostic/fix/close/delete. |
| POD hover cache cap | #8122 locked POD cache cap/prune behavior and active document path eviction. |
| File-watcher bulk churn | #8124 locked file-watcher debounce pressure drain and delete cleanup for document/session/index state. |
| Trend renderer | #8126 added `cargo xtask memory-trends render` and committed `docs/project/status/memory_plateau_trends.md`. |
| Regression issue template and triage | #8127 added the GitHub Memory Regression issue template and linked the failure triage path from memory status docs. |
| Formatting subprocess smoke | #8130 added repeated perltidy `format_file` coverage for subprocess output, cache length, and temp-state retention. |
| DAP start/stop smoke | #8132 added repeated DAP bridge child-process start/stop coverage and PID liveness checks. |
| Pressure-signal inventory guard | #8141 required every retained-state inventory row to name a pressure counter or retained-process signal and extended `cargo xtask check-memory-lifecycle-policy`. |

## Closeout Criteria

| Criterion | Evidence |
|-----------|----------|
| Every retained owner has owner, key, bound, cleanup event, pressure signal, and regression surface. | `docs/large-workspaces/RETAINED_STATE_INVENTORY.md` has first-class columns for the required fields, enforced by `cargo xtask check-memory-lifecycle-policy`. |
| Every high-risk retained-state surface has a focused scenario. | `RETAINED_STATE_INVENTORY.md` lists document churn, workspace-symbol churn, workspace index reindex, diagnostics, POD hover, stream sessions, file watcher churn, workspace-folder removal, formatting subprocess, and DAP start/stop coverage. |
| Process-lifecycle surfaces have retained-process smoke coverage. | Formatting subprocess coverage is in #8130; DAP child-process start/stop coverage is in #8132. |
| Memory receipts can be trended. | `cargo xtask memory-trends render` updates `docs/project/status/memory_plateau_trends.md` from plateau summaries, receipts, and committed baselines. |
| Failure triage is documented. | `.github/ISSUE_TEMPLATE/memory_regression.yml` captures scenario, commit, artifacts, slope, counters, owner, and lifecycle; `docs/project/status/memory_plateau.md` summarizes the triage path. |
| Policy prevents inventory drift. | `cargo xtask check-memory-lifecycle-policy` checks close/delete semantics, generation guards, memory counters, receipt registration, receipt schema fields, and retained-state inventory pressure signals. |
| No covered memory-control item points at accidental deferred coverage. | The retained-state scenario table uses current coverage language for completed surfaces. Additional work should be tied to new retained owners or failing receipts. |

## Response Path

When memory behavior breaks:

1. Open a Memory Regression issue from the GitHub template.
2. Attach the plateau receipt, trend row, CSV/server log, or retained-state test output.
3. Compare `MemoryStateSnapshot` and `RuntimePressureSnapshot`.
4. Identify the owner from `RETAINED_STATE_INVENTORY.md`.
5. Add or tighten a failing focused regression before patching.
6. Patch only the owner's cleanup, bound, counter, or retained-process path.

Do not add a broad RSS workload or speculative cleanup PR unless the focused
evidence shows the existing receipt/test surface cannot isolate the owner.

## Maintenance Rule

Any future PR that adds or changes a long-lived map, cache, debouncer, queue,
session, or subprocess owner must update the retained-state inventory with:
owner, key type, byte-risk, bound or cleanup event, pressure counter or retained
process signal, and regression test or receipt.

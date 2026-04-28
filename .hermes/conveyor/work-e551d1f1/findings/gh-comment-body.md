## Verification Agent Findings

**Confidence: Medium** — The research is broadly correct architecturally but contains a significant factual error about `diagnostics_stats.rs` and mischaracterizes the LSP operation instrumentation story.

---

### Confirmed Findings

1. **`workspace_stats.rs` is a genuine stub** — 9 lines, only `println("[stub] workspace-stats not yet implemented")`. Correctly identified.

2. **`memory.rs` is a genuine stub** — identical stub pattern. Correctly identified.

3. **`SloTracker` has 8 `OperationType` variants** — `IndexInitialization`, `IncrementalUpdate`, `DefinitionLookup`, `Completion`, `Hover`, `FindReferences`, `WorkspaceSymbols`, `FileIndexing`. Confirmed at `crates/perl-workspace-index/src/slo/mod.rs:78-96`.

4. **`SloTracker` tracks p50/p95/p99 + SLO compliance** — `SloStatistics` struct at line 169. Confirmed.

5. **`ProductionIndexCoordinator::statistics()` exposes `slo_stats`** — returns `HashMap<OperationType, SloStatistics>` plus `all_slos_met`. Confirmed at `production_coordinator.rs:468`.

6. **`release_health.rs` is fully implemented** — 589 lines, reads debt-ledger + CI baseline, 7 tests. Confirmed.

7. **`parser_stats.rs` has `slowest` vec** — top 5 by mean. Confirmed.

8. **`ratchet.rs` and `stable_wins.rs` fully implemented** — 4-layer ratchet model confirmed.

---

### Corrected Findings

**1. `diagnostics_stats.rs` is NOT active — it is a stub**

Research claimed: "Active — Diagnostics scorecard"

Actual (`xtask/src/tasks/metrics/diagnostics_stats.rs:1-9`):
```rust
//! [stub] Diagnostics accuracy and latency statistics subcommand.
pub fn run() -> Result<()> {
    println!("[stub] diagnostics-stats not yet implemented");
    Ok(())
}
```

Three of eight metric xtask modules are stubs (`workspace_stats`, `memory`, `diagnostics_stats`), not two.

**2. Per-file phase timing is NOT entirely benchmark-only**

Research claimed phase timing is "only in the dedicated benchmark runner". However, `sweep_stats.rs` reads corpus sweep receipts that include `phase_timings: {discovery_ms, file_io_ms, parse_ms, total_ms}` and `slowest_files: [{path, parse_duration_ms, line_count}]`. The 1.3.0 sweep receipt schema already provides per-file parse timing and phase breakdowns.

**3. Schema-vs-computation distinction**

Research says `lsp_stats.rs` "already computes" pass rates. The `UxMetrics` schema defines these as `Option<f64>` fields, but actual values require running `editor_intelligence_scorecard` tests first to produce `.ci/metrics/editor_ux.json`. The xtask reads a pre-existing receipt — it doesn't run the tests itself.

---

### New Findings

1. **`sweep_stats.rs` already surfaces per-file parse times** — the research did not mention this module in the phase timing section. The "top-N slowest parser files" goal may be partially met by enhancing this existing command.

2. **Plan Risk 1 is overstated** — the plan says wiring `SloTracker` to xtask requires "thread-safe access" with "snapshot mechanism". But `ProductionIndexCoordinator::statistics()` returns an owned `CoordinatorStatistics` struct — no locks held in the return value. The integration is simpler than the risk section suggests.

3. **Scope is broader than 9 recommendations** — the 7 scorecard design implies 7 `.ci/metrics/<subsystem>.json` files. `diagnostics_stats.rs` stub means the diagnostics scorecard (#4065) isn't even started. The `@INC` conformance matrix needs a new test harness. Phase 2 items (correctness/conformance) depend on labelled corpus fixtures still being built.

---

### Verification Methodology

Read all 8 metric xtask modules directly; verified `SloTracker` source at `crates/perl-workspace-index/src/slo/mod.rs`; verified `ProductionIndexCoordinator::statistics()` at `production_coordinator.rs:468`; read metrics README; checked gold fixture loading. The codebase verification was thorough.

---

*verification-agent*
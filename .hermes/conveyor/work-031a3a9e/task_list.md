# Task List — work-031a3a9e

## Phase 1: Mutation Notes (Documentation Only)
- [ ] Add doc comment to `collect_per_crate_mutation()` explaining mutation counts vs. scores
- [ ] Add `QUALITY_MUTATION_NOTES` marker block to `quality.md`

## Phase 2: Per-Subsystem Latency
- [ ] Add `collect_latency_by_subsystem()` function in `quality.rs`
- [ ] Add `format_latency_table()` function in `quality.rs`
- [ ] Extend `generate_quality_status()` to populate `PERFORMANCE_BY_SUBSYSTEM` block
- [ ] Add `PERFORMANCE_BY_SUBSYSTEM` marker block to `quality.md`

## Phase 3: Flaky Test Tracker via Debt-Ledger
- [ ] Extend `.ci/debt-ledger.yaml` schema with `failure_count` and `last_failed_at` fields
- [ ] Create `.ci/scripts/update-flaky-tracker.py` executable script
- [ ] Extend `generate_quality_status()` to populate `FLAKY_TEST_BULLETS` block
- [ ] Add `FLAKY_TEST_BULLETS` marker block to `quality.md`

## Phase 4: Per-Subsystem Test Counts
- [ ] Create `.ci/subsystem-mapping.yaml` mapping all 124 crates to subsystems
- [ ] Add `collect_subsystem_test_counts()` function
- [ ] Add validation that all 124 crates map to exactly one subsystem
- [ ] Extend `generate_quality_status()` to populate `SUBSYSTEM_TEST_BULLETS` block
- [ ] Add `SUBSYSTEM_TEST_BULLETS` marker block to `quality.md`

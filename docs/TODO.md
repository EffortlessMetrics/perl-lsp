# TODOs & Missing Features

> **Last Updated**: 2026-02-17
> **Sources of truth**: `docs/ROADMAP.md` (plans), `docs/CURRENT_STATUS.md` (metrics), `features.toml` (capabilities)
> **Rule**: If this file conflicts with those sources, update this file (not the sources).

---

## How to Use This List

- Treat this as an **actionable backlog**, not a status report.
- Do **not** add metrics here; metrics live in `docs/CURRENT_STATUS.md`.
- For LSP capability truth, update `features.toml` and run `just status-update`.

---

## Now (v0.9.1 Close-Out) - TODOs

### Workspace Index State Machine

- [x] Define explicit indexing states + transitions (Idle -> Scanning -> Indexing -> Ready -> Error)
- [x] Wire transitions into workspace indexing and file-change flows
- [x] Add early-exit heuristics and performance caps (target: <100ms initial, <10ms incremental)
- [x] Add instrumentation (state durations, early-exit reasons, transition counts)
- [x] Add targeted tests + benchmarks (small/medium/large workspaces)
- [x] Document invariants and failure modes (docs + inline commentary)
- [x] Capture receipts (ci-gate + targeted tests/benchmarks)

### Documentation Cleanup (missing_docs + module-level docs)

- [x] Run `cargo test -p perl-parser --features doc-coverage --test missing_docs_ac_tests` and capture receipts
- [x] Add or verify module-level docs for public modules (perl-parser + other public crates)
- [x] Ensure `cargo doc --no-deps -p perl-parser` is clean
- [x] Align wording across `START_HERE.md`, `CURRENT_STATUS.md`, `ROADMAP.md`, `CHANGELOG.md`

### Release Notes + Doc Alignment

- [x] v0.9.1 release notes draft (CHANGELOG + release summary)
- [x] Ensure `features.toml` and capability snapshots remain consistent
- [x] Verify `docs/CURRENT_STATUS.md` narrative matches receipts

---

## Missing Features (Derived from `features.toml`)

### LSP (Not Advertised / Preview)

- **`lsp.notebook_document_sync`** (preview, not advertised)
  - [x] Implement notebook document sync handlers
  - [x] Add capability gating and tests

- **`lsp.notebook_cell_execution`** (preview, not advertised)
  - [x] Track execution summary metadata
  - [x] Add tests for notebook execution summary updates

### DAP (Preview / Not Advertised)

- **`dap.breakpoints.hit_condition`** (preview, not advertised)
  - [x] Validate hit-count parsing and runtime counter behavior
  - [ ] Add dedicated E2E fixture coverage for multi-hit workflows

- **`dap.breakpoints.logpoints`** (preview, not advertised)
  - [x] Implement logMessage parsing and output emission path
  - [ ] Add dedicated E2E fixture for output+continue semantics

- **`dap.exceptions.die`** (preview, not advertised)
  - [x] Implement `setExceptionBreakpoints` filter handling
  - [ ] Add E2E fixture proving stop behavior changes when enabled

---

## Next (v0.9.1 Readiness) - TODOs

- [ ] Stability statement (versioning rules)
- [ ] Packaging stance (what ships; supported platforms)
- [ ] Benchmark receipts committed under `benchmarks/results/`
- [ ] Upgrade notes from v0.8.x -> v0.9.x
- [ ] Merge-gate work unblocked by CI pipeline cleanup (#211 -> #210)

---

## Later (Post v0.9.x)

- [ ] Native DAP completeness: attach, variables/evaluate, safe eval
- [ ] Full LSP 3.18 compliance audit vs spec (add missing catalog items to `features.toml`)
- [ ] Package manager distribution (Homebrew/apt/etc.)

---

## Quick Receipts / Checks

```bash
# Gate + metrics
nix develop -c just ci-gate
just status-check

# Missing docs validation
cargo test -p perl-parser --test missing_docs_ac_tests
cargo doc --no-deps -p perl-parser
```

---

## Notes

- If a TODO becomes a claim (\"done\", \"complete\"), move it into `docs/CURRENT_STATUS.md` with receipts.
- If a capability is missing, update `features.toml` first; then regenerate computed docs via `just status-update`.

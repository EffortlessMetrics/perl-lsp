# Path to Full Implementation (Updated)
## Perl LSP Project - Updated Roadmap to Full Implementation

**Document Version:** 1.1
**Report Date:** 2026-01-28 (updated from 2026-01-24)
**Current Release:** v0.9.0 (released 2026-01-18)
**Next Release:** v0.9.1 (in progress)
**Sources (canonical):**
- `docs/CURRENT_STATUS.md` (last updated 2026-01-23)
- `docs/ROADMAP.md` (status 2026-01-23)
- `docs/SEMANTIC_ANALYZER_STATUS.md` (last updated 2026-01-22)
- `features.toml` (LSP capability catalog)

---

## Executive Summary

The project remains production-ready for the core parser + LSP scope, with v0.9.0 released on 2026-01-18. v0.9.1 is in progress, and v1.0 is gated by CI pipeline cleanup (#211) and formal merge-blocking gates (#210). The semantic analyzer now has full NodeKind coverage (Phase 2-6 complete), but several advanced semantic features (closures, import resolution, multi-file flow) are explicitly deferred per the roadmap. The Debug Adapter Protocol (DAP) is usable in Phase 1 via a native adapter CLI plus BridgeAdapter, but attach/variables/evaluate remain pending.

Key points aligned with canonical sources:
- **LSP coverage and protocol compliance** are tracked by `features.toml` and surfaced in `CURRENT_STATUS.md` (100% user-visible, 53/53; 88/88 protocol including plumbing).
- **Semantic Analyzer** Phase 2-6 complete, with deferred closure capture and advanced multi-file analysis.
- **v0.9.1 work** remains: index state machine, documentation cleanup, and version bumps (Cargo.toml entries are still 0.8.8).
- **Critical Path to v1.0**: #211 -> #210 -> v1.0 release tasks (stability statement, benchmarks, packaging stance, upgrade notes).

---

## Reality Check: Gap Corrections vs Prior Report

These corrections address gaps or overstatements from the previous report:

1. **Semantic Analyzer completeness**
   - [OK] Phase 2-6 complete (all NodeKind handlers). 
   - [TODO] Advanced semantics still deferred: closure capture, import symbol resolution beyond basic parsing, cross-file flow analysis. (See `docs/SEMANTIC_ANALYZER_STATUS.md`.)

2. **DAP status**
   - [OK] Native adapter CLI and BridgeAdapter are present.
   - [TODO] Attach, variables/evaluate, and safe evaluation are explicitly deferred.

3. **LSP feature coverage**
   - Coverage is **computed** from `features.toml` and not a subjective percentage; any partial or experimental feature should be reflected by `maturity`/`advertised` flags. If a feature is only partially implemented, it must be downgraded in `features.toml` or marked as preview.

4. **Versioning**
   - v0.9.0 has been released, but Cargo.toml versions remain at 0.8.8 (plus perl-dap 0.1.0). A version bump to 0.9.1 is still pending.

---

## Major Progress Since the Jan 24 Report

No new receipts were identified beyond the canonical docs updated between 2026-01-22 and 2026-01-23. This update consolidates and corrects the earlier report against those canonical sources. Any new claims should be backed by `just ci-gate`, `scripts/ignored-test-count.sh`, or capability snapshots (see `docs/CURRENT_STATUS.md`).

---

## Current Status Snapshot (Receipt-Based)

| Component | Status | Evidence | Notes |
| --- | --- | --- | --- |
| **perl-parser** | Production | `just ci-gate` | ~100% Perl 5 syntax, incremental updates ~931ns (CURRENT_STATUS) |
| **perl-lsp** | Production (advertised subset) | `features.toml` + tests | 53/53 user-visible features, 88/88 protocol incl. plumbing |
| **perl-dap** | Phase 1 | manual smoke | Native adapter CLI; BridgeAdapter library; attach/eval pending |
| **Semantic Analyzer** | Phase 2-6 complete | `just ci-gate` | Full NodeKind handlers; closures/imports deferred |
| **Docs (perl-parser)** | Ratcheted | missing_docs=0 | Workspace-wide enforcement is a separate decision |
| **Security** | Hardened | doc claims | Path traversal + injection hardening complete |

**Note:** Some older docs (e.g., `docs/START_HERE.md`) contain stale metrics. The canonical sources above should be treated as truth.

---

## Critical Path to v1.0 (Updated)

Critical Path (sequential):

- Week 1-3: Issue #211 - CI Pipeline Cleanup (3 weeks)
  - Timing infrastructure
  - Baseline measurement
  - Workflow consolidation
  - Feature branch validation
  - Master branch enablement
  - Status: BLOCKER
- Week 4-11: Issue #210 - Merge-Blocking Gates (8 weeks)
  - Gate registry + receipts
  - Staged pre/post-merge gates
  - Check-run integration
  - Status: BLOCKED by #211
- Week 12: v1.0.0 Release
  - Stability statement
  - Packaging stance
  - Benchmark publication
  - Upgrade notes
  - Status: PLANNED

**Evidence:** #211 timing and cost targets in `docs/CI_COST_TRACKING.md`; #210 plan in `ISSUE_210_IMPLEMENTATION_PLAN.md`.

---

## Remaining Work by Category

### v0.9.1 Deliverables (High Priority)

| Task | Scope | Effort | Notes |
| --- | --- | --- | --- |
| **Index state machine** | Workspace indexing transitions + early exit | 4-6 hours | Target <100ms initial, <10ms incremental (ROADMAP) |
| **Documentation cleanup** | Reduce missing_docs violations | 4-6 hours | perl-parser baseline is 0; other crates pending |
| **Version bump to 0.9.1** | Update Cargo.toml versions | 2-4 hours | Root and crate manifests still at 0.8.8 |
| **Benchmark publication** | Commit results under `benchmarks/results/` | 2-4 hours | Framework exists; results not committed |

### v1.0.0 Deliverables (Critical Path)

| Task | Scope | Effort | Notes |
| --- | --- | --- | --- |
| **Stability statement** | docs/STABILITY.md + release notes | 8-12 hours | Required for v1.0 release |
| **Packaging stance** | distro/installer guidance | 4-8 hours | Homebrew/apt posture and support matrix |
| **Benchmark publication** | finalize results | 8-16 hours | Publish in `benchmarks/results/` |
| **Upgrade notes** | v0.8.x -> v1.0 | 4-8 hours | docs/UPGRADING.md updates |
| **Tier-1 CI validation** | full platform coverage | 8-16 hours | Ensure receipts on all Tier-1 platforms |

### Post-v1.0 (Deferred by Roadmap)

| Category | Scope |
| --- | --- |
| **DAP Phase 2/3** | attach, variables/evaluate, native adapter completeness |
| **Closure analysis** | capture + upvalue tracking for anonymous subs |
| **Import resolution** | Exporter.pm tracking, symbol availability |
| **Advanced multi-file** | workspace call graph, cross-file type flow |
| **Lexer optimizations** | SIMD, regex caching (#193) |

---

## Implementation Gaps (Concrete and Actionable)

These are the gaps that must be addressed to claim "full implementation," mapped to the correct source of truth and actionable work items.

### 1) Semantic Analyzer (Deferred Advanced Features)
- **Closure capture**: implement lexical binding + upvalue tracking (see deferred section in `docs/SEMANTIC_ANALYZER_STATUS.md`).
- **Import resolution**: track Exporter.pm symbols and propagate availability to semantic analysis.
- **Cross-file analysis**: build workspace call graph and variable flow propagation.

### 2) DAP (Production Completeness)
- **Attach**: implement attach flow in native adapter.
- **Variables/evaluate**: safe eval path and DAP variables tree.
- **Security**: maintain injection and path hardening in all commands.

### 3) CI/CD (Release Gatekeeping)
- **Issue #211**: consolidate workflows and reduce CI spend.
- **Issue #210**: implement gate registry + receipts + check-run integration.

### 4) Versioning + Release Hygiene
- **Version bump**: update Cargo.toml versions to 0.9.1 for next dev cycle.
- **Benchmarks**: publish results to the repo, not just framework documentation.

---

## Risk Assessment (Aligned to Canonical Docs)

| Risk | Probability | Impact | Mitigation |
| --- | --- | --- | --- |
| **CI pipeline cleanup overrun (#211)** | Medium | High | Incremental consolidation + feature branch validation |
| **Merge-gate complexity (#210)** | Medium | Medium | Staged rollout, local-first verification |
| **LSP test flakiness** | Medium | Medium | Keep adaptive timeouts + receipts; track ignored tests |
| **Index state machine scope creep** | Medium | High | Implement minimal state transitions first, benchmark later |
| **Docs drift** | Medium | Medium | Run `just status-update` and enforce receipts |

---

## Success Criteria (Receipt-Based)

### v0.9.1 Success
- [ ] Index state machine with <100ms initial, <10ms incremental
- [ ] Documentation violations < 200 (outside perl-parser baseline)
- [ ] Versions bumped to 0.9.1 in all Cargo.toml files
- [ ] `just ci-gate` passing

### v1.0.0 Success
- [ ] #211 complete (CI pipeline cleanup)
- [ ] #210 complete (merge-blocking gates + receipts)
- [ ] Stability statement published
- [ ] Packaging stance documented
- [ ] Benchmarks committed to `benchmarks/results/`
- [ ] Upgrade notes published
- [ ] CI passing on all Tier-1 platforms

---

## Recommended Next Actions (Immediate)

1. **Start Issue #211**: timing infra -> baseline -> workflow consolidation.
2. **Version bump to 0.9.1**: update root + crate manifests.
3. **Index state machine**: implement state transitions and early exit.
4. **Docs cleanup**: reduce remaining `missing_docs` violations outside perl-parser.

---

## Reference Links

- [CURRENT_STATUS.md](../docs/CURRENT_STATUS.md)
- [ROADMAP.md](../docs/ROADMAP.md)
- [SEMANTIC_ANALYZER_STATUS.md](../docs/SEMANTIC_ANALYZER_STATUS.md)
- [ISSUE_210_IMPLEMENTATION_PLAN.md](../ISSUE_210_IMPLEMENTATION_PLAN.md)
- [CI_COST_TRACKING.md](../docs/CI_COST_TRACKING.md)

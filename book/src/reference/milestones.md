# Milestones

> **Source of Truth**: GitHub Milestones at https://github.com/EffortlessMetrics/perl-lsp/milestones
>
> This document provides context and links. For live issue counts, check GitHub directly.

---

## Active Milestones

### v0.9.1: Post-Release Optimization

**Status**: Active (local; see GitHub milestones for live counts)
**Goal**: Close out v0.9.1 hardening and documentation cleanup.

**Exit Criteria**:
- See `ROADMAP.md` v0.9.1 section (index state machine, documentation cleanup, test debt)

**Constraints**:
- CI pipeline cleanup (#211) blocks merge gates (#210)

---

### v0.10.0 and Beyond

**Status**: Planning
**Goal**: Continue refinement toward the v0.15.0 Stability Contract.

**Focus Areas**:
- Moo/Moose semantic depth improvements
- Native DAP enhancements (variables/evaluate)
- Stability goal refinement for v0.15.0 contract
- Benchmark publication under benchmarks/results/

See [ROADMAP.md](ROADMAP.md) for the full release plan.

---

## Released Milestones

### v0.9.0: Semantic-Ready

**Status**: Released (2026-01-18)
**Goal**: A release that external users can try without reading internal docs.

**Exit Criteria**:
- `nix develop -c just ci-gate` green on MSRV
- `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
- README / CURRENT_STATUS / ROADMAP agree (no unbacked claims)
- `cargo install --path crates/perl-lsp` works cleanly
- Capability snapshot remains stable
- Release notes match advertised capabilities

**Historical blockers**:
- [#211](https://github.com/EffortlessMetrics/perl-lsp/issues/211) - CI Pipeline Cleanup
- [#210](https://github.com/EffortlessMetrics/perl-lsp/issues/210) - Merge-Blocking Gates
- [#143](https://github.com/EffortlessMetrics/perl-lsp/issues/143) - unwrap() panic safety

[View all v0.9.0 issues](https://github.com/EffortlessMetrics/perl-lsp/milestone/1)

---

## Phase Labels

Issues are tagged with phase labels to track the "good Perl experience" progression:

| Label | Description | Focus |
|-------|-------------|-------|
| `phase:stability` | Boundedness/hang hardening | Parser won't melt on ugly Perl |
| `phase:single-file` | Single-file semantic experience | Defs, hovers, completions in-file |
| `phase:workspace` | Multi-file workspace indexing | Cross-file navigation |

---

## Query Shortcuts

```bash
# v0.9.0 blockers only
gh issue list --milestone "v0.9.0: Semantic-Ready" --label "v0.9-blocker"

# All stability work
gh issue list --label "phase:stability"

# All v0.9.0 issues
gh issue list --milestone "v0.9.0: Semantic-Ready"

# All open issues
gh issue list --state open
```

---

## Milestone Lifecycle

1. **Active**: Currently accepting work
2. **Frozen**: No new issues; only fixing blockers
3. **Released**: Tagged and shipped
4. **Archived**: Closed, no longer relevant

When a milestone is released:
1. Close the milestone
2. Move any unresolved issues to the next milestone
3. Tag the release
4. Update ROADMAP.md

---

## Related Documentation

- [ROADMAP.md](ROADMAP.md) - High-level release planning
- [CURRENT_STATUS.md](CURRENT_STATUS.md) - Computed metrics
- [issues/corpus/gaps/](issues/corpus/gaps/) - Corpus coverage gaps

<!-- Last Updated: 2026-01-27 -->

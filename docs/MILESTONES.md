# Milestones

> **Source of Truth**: GitHub Milestones at https://github.com/EffortlessMetrics/tree-sitter-perl-rs/milestones
>
> This document provides context and links. For live issue counts, check GitHub directly.

---

## Active Milestones

### v0.9.0: Semantic-Ready

**Status**: Active
**Goal**: A release that external users can try without reading internal docs.

**Exit Criteria**:
- `nix develop -c just ci-gate` green on MSRV
- `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
- README / CURRENT_STATUS / ROADMAP agree (no unbacked claims)
- `cargo install --path crates/perl-lsp` works cleanly
- GA-lock capability snapshot remains stable
- Release notes match advertised capabilities

**Blockers** (must resolve before release):
- [#211](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/211) - CI Pipeline Cleanup
- [#210](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/210) - Merge-Blocking Gates
- [#143](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/143) - unwrap() panic safety

**Effort Estimate**: ~24 hours focused work (~1 week calendar time)

[View all v0.9.0 issues](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/milestone/1)

---

### v1.0.0: Boring Promises

**Status**: Queued (after v0.9.0)
**Goal**: Freeze the surfaces you're willing to support.

**Exit Criteria**:
- v0.9.0 released and stable
- Capability snapshot + docs aligned
- Benchmarks published under benchmarks/results/
- Upgrade notes exist from v0.8.x → v1.0

**Deliverables**:
1. Stability statement (what "GA-lock" means)
2. Packaging stance (binaries, crates, platforms)
3. Benchmark publication

**Effort Estimate**: ~40-80 hours after v0.9.0

[View all v1.0.0 issues](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/milestone/2)

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

# All v1.0.0 issues
gh issue list --milestone "v1.0.0: Boring Promises"
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

<!-- Last Updated: 2026-01-08 -->

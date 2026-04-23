4. **Hierarchical memory is not wired** — \`EstimateSize\` trait exists on \`BoundedLruCache\` but \`memory.rs\` doesn't break down by clangd-style categories.

5. **\`@INC\` conformance matrix and diagnostics/editor correctness scorecards** are stubs or missing entirely.

6. **Release-health model is implemented** in \`release_health.rs\`.

### Proposed Approach

Phase 1 (weeks 1–2): Wire \`SloTracker\` to \`workspace_stats.rs\` + add cold/warm/incremental tagging + implement top-N slowest-file reports. These give developers rust-analyzer/pyright-style profiling tools immediately.

Phase 2 (weeks 2–4): Implement \`@INC\` conformance matrix (#4067) + diagnostics correctness (#4065) + editor intelligence scorecard (#4066).

Phase 3 (weeks 4–6): Implement hierarchical memory report (#4070) + enhance release-health dashboard.

### Top Risks

1. **\`SloTracker\` threading model** — serializing statistics snapshot from xtask requires careful handling of the lock-free or atomic snapshot approach.
2. **\`@INC\` conformance matrix needs a representative fixture corpus** — without real files exercising each resolution path, the numbers are meaningless.
3. **Phase-timing overhead in hot paths** — latency-sensitive LSP operations (completion, hover) must not be slowed by timing instrumentation.

### Scope

Covers: workspace indexing observability, cold/warm/incremental latency tracking, per-file heavy-hitter reports, \`@INC\` conformance matrix, diagnostics and editor intelligence correctness, hierarchical memory accounting, release-health enhancement.

Does not cover: LSP wire protocol changes, VSCode extension changes, opt-in telemetry collection (design only), \`features.toml\` catalog changes.

---
*Research findings produced by research-agent for work-e551d1f1*"
  1.8s
  ┊ 💬 Comment posted successfully via gh CLI. Let me log friction points:
  ┊ 💻 $         cd /home/hermes/repos/hermes && python3 conveyor/gates.py friction work-e551d1f1 "gates.py read-issue command not found (unrecognized), had to use gh CLI directly to fetch issue data"  0.6s
  ┊ 💻 $         cd /home/hermes/repos/hermes && python3 conveyor/gates.py friction work-e551d1f1 "gates.py post-comment failed with BadRequestError HTTP 400, had to use gh issue comment CLI directly"  0.6s
## Summary

**Research agent completed for `work-e551d1f1`**

### Artifacts Produced

1. **`research_analysis.md`** — Analysis of issue #4099 (reference-model findings from rust-analyzer, gopls, pyright, clangd) mapped onto perl-lsp's existing codebase. Key findings:
   - `SloTracker` in `perl-workspace-index` already tracks 8 operation types with latency percentiles — but `workspace_stats.rs` is an empty stub
   - Cold/warm/incremental regime separation is missing
   - Hierarchical memory accounting exists via `EstimateSize` trait but isn't wired to `memory.rs`
   - `@INC` conformance matrix and diagnostics/editor correctness scorecards are stubs or missing
   - Release-health dashboard is implemented

2. **`initial_plan.md`** — Phased implementation plan:
   - Phase 1 (weeks 1–2): Wire `SloTracker` → `workspace_stats.rs`, cold/warm/incremental tagging, top-N slowest files
   - Phase 2 (weeks 2–4): `@INC` conformance matrix, diagnostics correctness, editor intelligence scorecard
   - Phase 3 (weeks 4–6): Hierarchical memory report, release-health enhancements

3. **GitHub comment** posted to issue #4099 with findings summary: https://github.com/EffortlessMetrics/perl-lsp/issues/4099#issuecomment-4269449250

### Friction Logged
- `gates.py read-issue` command not found — used `gh issue view` directly
- `gates.py post-comment` returned HTTP 400 — used `gh issue comment` directly
__TOKEN_JSON__{"total_tokens": 905163, "input_tokens": 60376, "output_tokens": 9830, "cache_read_tokens": 834957, "cache_write_tokens": 0, "reasoning_tokens": 0, "api_calls": 23, "estimated_cost_usd": 0.0, "response_chars": 1400, "prompt_chars": 3667, "model": null, "provider": null}
# Large-Workspace Contributor Guides

Practical guides for contributors working with or testing against large
Perl workspaces (5 000–10 000+ files).

| Guide | When to read it |
|-------|----------------|
| [TESTING_GUIDE.md](TESTING_GUIDE.md) | Generating test workspaces, writing large-workspace tests, performance baselines |
| [PROFILING_GUIDE.md](PROFILING_GUIDE.md) | CPU flamegraphs, heap profiling, criterion benchmarks |
| [MEMORY_PATTERNS.md](MEMORY_PATTERNS.md) | How memory scales, cache behavior, common anti-patterns |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Diagnosing slowdowns, stale symbols, index degradation |

## Quick Reference

```bash
# Generate a 5 000-file synthetic workspace
bash scripts/gen-large-workspace.sh /tmp/big-workspace 5000

# Run the workspace index benchmarks
just bench

# Compare against baseline
just bench-compare

# Check running LSP server health
perllsp --health

# Debug log for troubleshooting
RUST_LOG=perl_lsp=debug,perl_workspace_index=debug perllsp --stdio 2>debug.log
```

## Related Documentation

- `docs/how-to/PERFORMANCE_TUNING.md` — end-user configuration for large workspaces
- `docs/reference/PERFORMANCE_SLO.md` — response time targets and degradation thresholds
- `docs/reference/PERFORMANCE_MONITORING.md` — automated regression alerts
- `docs/benchmarks/BENCHMARK_DESIGN.md` — benchmark architecture

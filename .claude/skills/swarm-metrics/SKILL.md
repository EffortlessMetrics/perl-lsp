---
description: Aggregate and interpret swarm metrics for status and report surfaces
---

# Swarm Metrics

Use this skill when you need to inspect `.ops-perl-lsp/swarm-metrics.jsonl`,
summarize recent swarm activity, or write status/report commentary that is
backed by the metrics file.

## Canonical Analyzer

Prefer the repo-native analyzer:

```bash
cargo xtask swarm-summary .ops-perl-lsp --since 24h --limit 10
cargo xtask swarm-summary .ops-perl-lsp --since 7d --limit 20
cargo xtask swarm-summary .ops-perl-lsp --since 24h --limit 10 --format json
```

## Reporting Patterns

- Use `--since 24h` for `/swarm-status` style check-ins.
- Use `--since 7d` for `/swarm-report` and weekly rollups.
- Use `--format json` when another tool needs machine-readable summary data.
- Call out the top event types, agent types, sessions, and worktree hotspots.
- Prefer the analyzer output over raw `tail`/`jq` snippets so the same
  grouping rules are used everywhere.

## Notes

- The metrics file is append-only and may contain mixed event shapes.
- Missing fields should be treated as `(none)` rather than dropped.
- If the analyzer reports zero matching entries, mention the window and the
  file path so the operator can distinguish "no data" from "wrong directory".

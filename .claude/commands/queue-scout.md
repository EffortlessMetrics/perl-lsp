---
description: Launch scouts across parser/LSP/DAP/security/test gaps
argument-hint: "[focus-area] e.g. 'parser', 'dap', 'issues', 'dead-code', 'all'"
---

# Queue Scout

Launch swarm-scout agents to find improvement slices. Focus: **$ARGUMENTS**

## Focus Areas

### `all` (default) — launch 10-15 scouts across everything:

| Count | Focus | Target |
|-------|-------|--------|
| 3-4 | Parser error buckets | `.ci/parser-corpus-baseline.json` top 5 categories |
| 2-3 | DAP test gaps | `perl-dap-value`, `perl-dap-shell`, `perl-dap-command-args`, `perl-dap-security` |
| 2-3 | Open issues | `gh issue list --state open` with clear acceptance criteria |
| 1-2 | Dead code / unused deps | `cargo machete`, `just dead-code`, `.ci/debt-ledger.yaml` |
| 1-2 | LSP feature polish | `features.toml` gaps, test coverage in `perl-lsp-*` |
| 1 | Ignored tests | `#[ignore]` tests whose blockers may be resolved |

### Specific focus — launch 3-5 scouts in that area:
- `parser` — error buckets only
- `dap` — DAP crate test gaps only
- `issues` — open GitHub issues only
- `dead-code` — unused deps, dead code, debt ledger
- `lsp` — LSP feature polish only
- `tests` — ignored tests, test coverage gaps

## Dispatch Pattern

For each scout:
```
Agent(
  subagent_type: "swarm-scout",
  prompt: "Focus area: <specific target>. Find ONE actionable improvement.",
  model: "sonnet",
  run_in_background: true,
  name: "scout-<focus>-<N>"
)
```

## After Scouts Complete

1. Collect all SLICE outputs
2. Check `files_touched` fields for overlaps
3. If two slices overlap, keep the higher-impact one, defer the other
4. Feed non-overlapping slices to swarm-builder agents
5. Update `.claude/swarm-state/swarm-queue.json` with active slices

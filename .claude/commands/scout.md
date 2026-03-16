---
description: Launch a single scout agent for a focus area
argument-hint: "<focus> e.g. 'parser', 'lsp', 'dap', 'docs', 'tests', 'devex', 'security', 'deps', 'perf'"
---

# Scout

Launch a focused exploration of **$ARGUMENTS** and produce GitHub issues for concrete findings.

## Dispatch Table

| Focus | Crate paths | Exploration targets |
|-------|-------------|---------------------|
| `parser` | `crates/perl-parser/`, `crates/perl-parser-core/`, `crates/perl-lexer/`, `crates/perl-tokenizer/` | Error buckets in `.ci/parser-corpus-baseline.json`, edge cases, TODO comments, missing test coverage |
| `lsp` | `crates/perl-lsp/`, `crates/perl-lsp-*/` | `features.toml` gaps, provider quality, test coverage, error handling, threading issues |
| `dap` | `crates/perl-dap/`, `crates/perl-dap-*/` | Protocol compliance, test gaps, error handling, security (command injection, path traversal) |
| `docs` | `docs/`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`, `crates/*/README.md` | Stale docs, missing docs, undocumented architecture, broken links, outdated examples |
| `tests` | `crates/*/tests/` | `#[ignore]` tests whose blockers may be resolved, low-coverage crates, missing edge cases, flaky tests |
| `devex` | `xtask/`, `scripts/`, `.ci/`, `justfile`, `Cargo.toml` | Build friction, slow commands, confusing errors, missing tooling, DX paper cuts |
| `security` | `deny.toml`, `crates/perl-dap-security/`, `crates/perl-dap-shell/` | Dependency advisories (`cargo audit`), unsafe code, command injection, path traversal, secrets in code |
| `deps` | `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/dependabot.yml` | Unused deps (`cargo machete`), outdated deps, duplicate versions, license issues |
| `perf` | `crates/perl-parser/`, `crates/perl-lexer/`, `crates/perl-workspace-index/` | Hot paths, unnecessary allocations, redundant clones, benchmark regressions, O(n^2) patterns |

## Process

1. Identify the focus area from `$ARGUMENTS` using the dispatch table above. If the argument does not match a known focus, treat it as a crate name and explore `crates/<argument>/`.

2. Spawn one `scout` agent for the focus area:

```text
Agent(
  subagent_type: "scout",
  prompt: "
    Focus: <focus area>
    Paths: <paths from dispatch table>

    Load /swarm-protocol for behavioral rules.
    Dedup-check completed slices, open issues, and open PRs.
    Find ONE actionable improvement with concrete file:line evidence.
    Write the finding as a GitHub issue via /scout-report.
  ",
  model: "sonnet",
  run_in_background: true,
  name: "scout-<focus>"
)
```

3. Collect the issue URL when the agent completes.

## Examples

```bash
/scout parser           # Explore parser crates for bugs and gaps
/scout dap              # Audit DAP protocol compliance
/scout lsp              # Check LSP feature completeness
/scout security         # Security-focused scan
/scout perl-refactoring # Scout a specific crate by name
```

## After Scouting

- Each finding becomes a GitHub issue via `/scout-report`
- Issues are labeled `swarm-discovered` or `swarm-architectural` for design decisions
- Builder agents can pick up issues directly
- Use `/queue-scout` for broad multi-area scouting instead of single-focus

## Relationship to /queue-scout

- `/scout <focus>` = single-area deep dive (3-5 minutes, 1 agent)
- `/queue-scout` = broad sweep across all areas (10-15 scouts in parallel)

Use `/scout` when you know where to look. Use `/queue-scout` when you want to discover what needs attention.

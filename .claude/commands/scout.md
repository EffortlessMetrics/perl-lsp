---
description: Launch a single scout agent for a focus area
argument-hint: "<focus> e.g. 'parser', 'lsp', 'dap', 'docs', 'tests', 'devex', 'security', 'deps', 'perf'"
---

# Scout

Launch a focused exploration of **$ARGUMENTS** and produce builder-ready GitHub issues.

## Dispatch Table

| Focus | Crate paths | Exploration targets |
|-------|-------------|---------------------|
| `parser` | `crates/perl-parser/`, `crates/perl-parser-core/`, `crates/perl-lexer/` | Error buckets, edge cases, corpus failures |
| `lsp` | `crates/perl-lsp/`, `crates/perl-lsp-*/` | `features.toml` gaps, provider quality, threading |
| `dap` | `crates/perl-dap/`, `crates/perl-dap-*/` | Protocol compliance, test gaps, security |
| `docs` | `docs/`, `README.md`, `CONTRIBUTING.md` | Stale/missing docs, broken links |
| `tests` | `crates/*/tests/` | Low-coverage crates, missing edge cases, flaky tests |
| `devex` | `xtask/`, `scripts/`, `.ci/`, `justfile` | Build friction, slow commands, DX paper cuts |
| `security` | `deny.toml`, `crates/perl-dap-security/` | Advisories, unsafe code, injection |
| `deps` | `Cargo.toml`, `Cargo.lock`, `deny.toml` | Unused deps, outdated, duplicates, licenses |
| `perf` | `crates/perl-parser/`, `crates/perl-workspace-index/` | Hot paths, allocations, O(n²) patterns |

## Scout Task Checklist

The scout agent MUST use TaskCreate to create this checklist and complete
each step IN ORDER before filing the issue. This ensures full context.

```
TaskCreate: "1. Dedup check — verify this isn't already filed or in-flight"
  - gh issue list --search "<topic>" --limit 10
  - gh pr list --search "<topic>" --limit 10
  - If duplicate exists, STOP and report "already tracked as #NNN"

TaskCreate: "2. Locate the code — find exact files and line numbers"
  - Grep/Glob for the relevant code paths
  - Read the specific functions involved
  - Record: file:line for every relevant location

TaskCreate: "3. Reproduce the problem — confirm the bug/gap with evidence"
  - For parser: find a corpus file that fails, extract minimal Perl snippet
  - For LSP: identify which request/response is wrong or missing
  - For perf: measure or estimate the impact
  - Record: exact error message, test command, or metric

TaskCreate: "4. Trace root cause — understand WHY it fails"
  - Read the code path that handles this case
  - Identify the specific function/branch that's wrong or missing
  - Record: root cause in one sentence + the code location

TaskCreate: "5. Design fix options — enumerate 2-3 approaches"
  - For each option: what changes, which files, what tradeoffs
  - Estimate effort: EASY (<2h), MEDIUM (2-8h), HARD (>8h)
  - Pick a recommendation with reasoning

TaskCreate: "6. Write test spec — what test proves the fix works"
  - Write the exact test code (Perl snippet or Rust test fn)
  - Specify the verification command: cargo test -p <crate> -- <test>
  - Record: test input, expected output

TaskCreate: "7. File builder-ready issue via /scout-report"
  - ALL previous tasks must be complete
  - Issue must contain: file:line, root cause, recommended fix, test spec
  - Invoke /scout-report with the complete findings
```

## Spawn Pattern

```text
Agent(
  subagent_type: "scout",
  prompt: "
    Focus: <focus area>
    Paths: <paths from dispatch table>
    Target: <specific topic to investigate>

    Use TaskCreate to create the 7-step scout checklist from /scout.
    Complete each step in order. Do NOT skip steps.
    After all 7 steps, invoke /scout-report to file the issue.
    If step 1 finds a duplicate, stop and report it.
  ",
  model: "sonnet",
  run_in_background: true,
  name: "scout-<focus>"
)
```

## Key Rule

**A scout that files an issue without completing all 7 steps has failed.**
The issue is the scout's deliverable, but only if it contains:
- Exact file:line locations (step 2)
- Reproduced evidence (step 3)
- Root cause (step 4)
- Fix options with recommendation (step 5)
- Test spec (step 6)

If any of these are missing, the builder will re-research — which defeats
the purpose of scouting.

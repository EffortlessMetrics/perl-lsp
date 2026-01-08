# Diff Scout Prompt

## Purpose

The Diff Scout analyzer produces a **review map** and identifies **hotspots** for PR forensics. It evaluates the **Scope** of changes, determining where the diff landed, what the riskiest changes are, and whether the work is primarily semantic or boilerplate.

**Quality Surface**: Scope (foundational for all surfaces)

## Required Inputs

Provide the following context to the analyzer:

### 1. Git Diff
```
<git_diff>
[Full or summarized git diff output]
</git_diff>
```

### 2. Commit History
```
<commit_history>
[Git log with commit messages, e.g., `git log --oneline <base>..HEAD`]
</commit_history>
```

### 3. File Change Histogram
```
<file_histogram>
[Output of `git diff --stat` or similar showing files and line deltas]
</file_histogram>
```

### 4. PR Metadata (optional but helpful)
```
<pr_metadata>
PR Number: <number>
Title: <title>
Stated Scope: <what PR description says it does>
</pr_metadata>
```

## Output Schema

The analyzer must produce output conforming to this YAML schema:

```yaml
analyzer: diff-scout
pr: <number>
timestamp: <ISO8601>
coverage: <github_only|github_plus_agent_logs|receipts_included>

review_map:
  - path: <file path>
    delta: <+X/-Y>
    category: <logic|test|config|docs|generated>
    risk: <high|medium|low>

hotspots:
  - path: <file path>
    reason: <why this is a hotspot>
    lines: <specific line ranges, e.g., "45-120">

commit_topology:
  feat: <count>
  fix: <count>
  test: <count>
  docs: <count>
  chore: <count>
  refactor: <count>
  other: <count>

semantic_ratio: <percentage, e.g., "75%">

findings:
  - id: <unique_id, e.g., "DS-001">
    severity: <P1|P2|P3|info>
    category: <scope_creep|large_change|high_risk_area|generated_dominant|unclear_commit_topology>
    summary: <one line>
    evidence:
      - anchor: <file:line or commit SHA>
        content: <excerpt>
    recommendation: <action>
    confidence: <high|medium|low>

summary:
  verdict: <pass|warn|fail>
  key_findings:
    - <bullet 1>
    - <bullet 2>

assumptions:
  - <what was assumed, e.g., "Commit messages follow conventional format">
```

## Key Questions Answered

1. **Where did the diff land?** - Which directories, modules, and file types received changes?
2. **What are the riskiest changes?** - Which files have high-risk modifications (core logic, security-sensitive, high churn)?
3. **Is this mostly semantic work or boilerplate?** - What percentage of changes are meaningful logic vs. generated code, config, or docs?
4. **Does the actual scope match the stated scope?** - Did files change outside the PR's claimed purpose?
5. **What's the commit topology?** - How do commits distribute across types (feat/fix/test/docs)?

## Risk Classification Rules

### High Risk
- Core parser/lexer logic changes
- Security-sensitive code (auth, validation, encoding)
- Public API surface modifications
- Files with high historical churn
- Large delta in single file (>200 lines changed)

### Medium Risk
- Test infrastructure changes
- Internal module refactoring
- Build/CI configuration
- Non-trivial documentation updates

### Low Risk
- Simple test additions
- Comment-only changes
- Formatting/whitespace
- Generated file updates (lockfiles, etc.)

## Category Classification

| Category | Criteria |
|----------|----------|
| `logic` | Core application code, business logic |
| `test` | Test files, test fixtures, test utilities |
| `config` | Build configs, CI configs, tooling configs |
| `docs` | Documentation, README, comments |
| `generated` | Lockfiles, auto-generated code, vendored deps |

## Example Input

```
<pr_metadata>
PR Number: 259
Title: Test harness hardening
Stated Scope: Fix BrokenPipe issues in LSP tests
</pr_metadata>

<commit_history>
a1b2c3d feat(lsp): add connection state tracking
e4f5g6h fix(lsp): handle BrokenPipe in shutdown
i7j8k9l test(lsp): add harness resilience tests
m0n1o2p docs: update test threading guidance
</commit_history>

<file_histogram>
 crates/perl-lsp/src/connection.rs | 145 ++++++++++++++++++++
 crates/perl-lsp/src/server.rs     |  32 ++---
 crates/perl-lsp/tests/harness.rs  |  89 +++++++++++++
 docs/LSP_TESTING.md               |  25 ++++
 4 files changed, 272 insertions(+), 19 deletions(-)
</file_histogram>
```

## Example Output

```yaml
analyzer: diff-scout
pr: 259
timestamp: 2025-01-07T12:00:00Z
coverage: github_only

review_map:
  - path: crates/perl-lsp/src/connection.rs
    delta: +145/-0
    category: logic
    risk: high
  - path: crates/perl-lsp/src/server.rs
    delta: +13/-19
    category: logic
    risk: medium
  - path: crates/perl-lsp/tests/harness.rs
    delta: +89/-0
    category: test
    risk: medium
  - path: docs/LSP_TESTING.md
    delta: +25/-0
    category: docs
    risk: low

hotspots:
  - path: crates/perl-lsp/src/connection.rs
    reason: New file with significant logic, connection state management is security-sensitive
    lines: "1-145"
  - path: crates/perl-lsp/src/server.rs
    reason: Modifications to server shutdown logic, error handling critical path
    lines: "78-110"

commit_topology:
  feat: 1
  fix: 1
  test: 1
  docs: 1
  chore: 0
  refactor: 0
  other: 0

semantic_ratio: "70%"

findings:
  - id: DS-001
    severity: info
    category: large_change
    summary: New connection.rs file adds 145 lines of state management logic
    evidence:
      - anchor: crates/perl-lsp/src/connection.rs:1-145
        content: "New file with ConnectionState enum and tracking"
    recommendation: Ensure comprehensive test coverage for new state machine
    confidence: high

summary:
  verdict: pass
  key_findings:
    - 70% semantic changes (logic + test infrastructure)
    - Primary hotspot is new connection state management
    - Scope aligns with stated goal (test harness hardening)

assumptions:
  - Commit messages follow conventional commit format
  - File extensions correctly indicate file purpose
```

## Trust Model

### Can Be Inferred (High Confidence)
- File categories from paths and extensions
- Line delta counts from diff
- Commit type distribution from conventional commit prefixes
- Semantic ratio from category classification

### Can Be Inferred (Medium Confidence)
- Risk levels (requires domain knowledge of codebase)
- Hotspot identification (heuristic-based)
- Scope alignment with stated goals

### Cannot Be Inferred
- Whether changes are correct (requires Verification Auditor)
- Whether boundaries are respected (requires Design Auditor)
- Actual test coverage (requires execution receipts)
- Historical churn data without repo access

### Red Flags to Note
- When more than 30% of changes are outside stated scope
- When semantic ratio is below 40% for a "feature" PR
- When a single file has >50% of total delta
- When commit messages don't follow conventional format

## Integration Notes

Diff Scout output feeds into:
- **Design Auditor**: Uses review map to focus boundary analysis
- **Verification Auditor**: Uses hotspots to prioritize test depth checks
- **Chronologist**: Uses commit topology for temporal narrative

Run Diff Scout first in the analyzer pipeline.

---
description: Write scout findings as a GitHub issue with structured context
argument-hint: "<title> <body-summary>"
---

# Scout Report

Convert scout findings into a GitHub issue. This is the standard output format for all scout-type skills.

## Required Context

The calling scout MUST have gathered:
1. **Title**: A concise description of the finding
2. **Category**: One of `bug`, `test-gap`, `dead-code`, `doc-gap`, `perf`, `security`, `devex`, `cleanup`
3. **Files**: Specific file paths with line numbers
4. **Impact**: Why this matters (severity, frequency, user-facing?)
5. **Suggested Approach**: Concrete next steps a builder agent can follow

## Issue Creation

```bash
gh issue create \
  --title "<category>: <concise title>" \
  --label "swarm-discovered" \
  --body "$(cat <<'EOF'
## Discovery

<What was found, why it matters>

## Category

<bug | test-gap | dead-code | doc-gap | perf | security | devex | cleanup>

## Files

<file paths with line numbers>

## Impact

<severity: low/medium/high, user-facing: yes/no, frequency: rare/common/always>

## Suggested Approach

<concrete steps a builder agent can follow without re-investigating>

## Agent

Discovered by `/scout` exploration.
EOF
)"
```

## Rules

- ONE issue per distinct finding. Do not bundle unrelated findings.
- If multiple findings in the same category are closely related (same root cause), combine into one issue.
- Always include enough context that a builder agent can start work without re-investigating.
- Use `--label swarm-architectural` instead of `--label swarm-discovered` if the finding needs a design decision from the user.
- Print the issue URL after creation so the caller can reference it.

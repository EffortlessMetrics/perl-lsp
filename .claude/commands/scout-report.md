---
description: Write scout findings as a GitHub issue with structured template
argument-hint: "<one-line title of the finding>"
---

# Scout Report

Write a GitHub issue documenting your scout finding. This is the scout's primary deliverable.

## Template

Create the issue using this exact structure:

```bash
gh issue create \
  --title "$ARGUMENTS" \
  --label "swarm-discovered" \
  --body "$(cat <<'ISSUE_EOF'
## Problem

_What is wrong or missing? Be specific - include file paths with line numbers, error messages, metric values._

<your evidence here>

## Options

_What are the possible approaches to fix this? List 2-3 with tradeoffs._

1. **Option A** - <description>. Tradeoff: <pro/con>.
2. **Option B** - <description>. Tradeoff: <pro/con>.
3. **Option C** - <description>. Tradeoff: <pro/con>.

## Recommendation

_Which option and why?_

<your recommendation>

## Acceptance Criteria

_How do we know this is done? Be concrete - test commands, metric thresholds, behavior changes._

- [ ] <criterion 1>
- [ ] <criterion 2>
- [ ] <criterion 3>

## Scope

- **Crate(s):** <which crates are affected>
- **Files:** <key files with paths>
- **Estimated size:** small / medium / large

---
_Filed by swarm-scout agent._
ISSUE_EOF
)"
```

## Rules

- ONE issue per distinct finding. Do not bundle unrelated findings.
- Fill in ALL sections. Do not leave placeholders.
- **Problem** must have file:line evidence, not just a vague description.
- **Options** must list at least 2 approaches with real tradeoffs.
- **Acceptance Criteria** must be verifiable.
- **Scope** must list affected crates and files so builders know what to claim.
- Always use label `swarm-discovered`, unless the finding needs a design decision. In that case use `swarm-architectural`.
- After creating the issue, print the issue URL so the coordinator can collect it.

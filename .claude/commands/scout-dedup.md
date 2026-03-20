---
description: Scout step 1 — check if this finding is already tracked
user-invocable: false
---

# Scout Dedup Check

Before investigating, verify this isn't already covered.

## Steps

1. Search open issues:
   ```bash
   gh issue list --state open --search "<your topic keywords>" --limit 10 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```

2. Search open PRs:
   ```bash
   gh pr list --state open --search "<your topic keywords>" --limit 10 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```

3. Search recently closed/merged (might be done):
   ```bash
   gh issue list --state closed --search "<your topic keywords>" --limit 5 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```

## Decision

- **Duplicate found**: TaskUpdate this step as completed, note the existing issue/PR number, STOP scouting. Report: "Already tracked as #NNN"
- **Related but different**: Note the related issue, continue. Your finding is a distinct slice.
- **No match**: Continue to step 2.

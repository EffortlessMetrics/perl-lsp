---
description: Scout step 1 — check if this finding is already tracked
user-invocable: false
---

# Scout Dedup Check

Before investigating, verify this isn't already covered.

## Steps

1. **Check local blockers ledger** for known active issues in this area:
   ```bash
   python3 -c "
   import yaml, sys
   data = yaml.safe_load(open('.ci/blockers.yaml'))
   topic = sys.argv[1].lower() if len(sys.argv) > 1 else ''
   for section in ['parser_blockers', 'lsp_limitations', 'ci_blockers']:
       for entry in data.get(section, []):
           if any(topic in str(v).lower() for v in entry.values()):
               print(f\"KNOWN: {entry['id']} -> issue {entry.get('issue','none')} ({entry.get('status','?')})\")
   " "<your topic keywords>" 2>/dev/null || echo "(blockers.yaml not yet available)"
   ```
   If a match with `status: filed` or `status: partial` is found, report it and consider stopping.

2. Search open issues:
   ```bash
   gh issue list --state open --search "<your topic keywords>" --limit 10 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__search_issues(query:"<your topic keywords> is:open repo:effortlessmetrics/perl-lsp-swarm")`

3. Search open PRs:
   ```bash
   gh pr list --state open --search "<your topic keywords>" --limit 10 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__search_pull_requests(query:"<your topic keywords> is:open repo:effortlessmetrics/perl-lsp-swarm")`

4. Search recently closed/merged (might be done):
   ```bash
   gh issue list --state closed --search "<your topic keywords>" --limit 5 --json number,title --jq '.[] | "#\(.number) \(.title)"'
   ```
   > **MCP alternative (web/no-gh sessions):** `mcp__github__search_issues(query:"<your topic keywords> is:closed repo:effortlessmetrics/perl-lsp-swarm")`

## Decision

- **Duplicate found**: TaskUpdate this step as completed, note the existing issue/PR number, STOP scouting. Report: "Already tracked as #NNN"
- **Related but different**: Note the related issue, continue. Your finding is a distinct slice.
- **No match**: Continue to step 2.

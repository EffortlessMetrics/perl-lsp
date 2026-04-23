#!/usr/bin/env bash
set -euo pipefail

# Show open issues missing required taxonomy labels
# Usage: bash scripts/gh/issues-needing-triage.sh [limit]
# Default limit: 500

limit="${1:-500}"

gh issue list --state open --limit "$limit" --json number,title,labels \
  --jq '
    .[]
    | .labels |= (map(.name))
    | {
        n: .number,
        t: .title,
        miss: (
          (if any(.labels[]; test("^type:")) then [] else ["type:*"] end) +
          (if any(.labels[]; test("^P[0-3]-")) then [] else ["P0–P3"] end) +
          (if any(.labels[]; test("^area:")) then [] else ["area:*"] end) +
          (if any(.labels[]; test("^status:")) then [] else ["status:*"] end)
        )
      }
    | select(.miss | length > 0)
    | "\(.n)\t\(.miss|join(","))\t\(.t)"
  ' | column -t -s $'\t'

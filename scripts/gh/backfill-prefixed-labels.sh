#!/usr/bin/env bash
set -euo pipefail

# Backfill prefixed labels from legacy labels (additive, non-destructive)
# Usage:
#   bash scripts/gh/backfill-prefixed-labels.sh          # Dry run (show what would be done)
#   bash scripts/gh/backfill-prefixed-labels.sh --apply  # Actually apply labels

apply=0
[[ "${1:-}" == "--apply" ]] && apply=1

if (( ! apply )); then
  echo "🔍 DRY RUN - showing what would be done (use --apply to execute)"
  echo ""
fi

gh issue list --state open --limit 1000 --json number,labels,title \
  --jq '.[] | [.number, (.labels|map(.name)|join("|")), .title] | @tsv' |
while IFS=$'\t' read -r num labels title; do
  add=()

  # type: map legacy labels to prefixed
  if [[ "$labels" != *"type:"* ]]; then
    [[ "$labels" == *"bug"* ]] && add+=("type:bug")
    [[ "$labels" == *"enhancement"* ]] && add+=("type:enhancement")
    [[ "$labels" == *"documentation"* ]] && add+=("type:docs")
  fi

  # priority: map legacy priority:* to P0-P3
  if [[ "$labels" != *"P0-"* && "$labels" != *"P1-"* && "$labels" != *"P2-"* && "$labels" != *"P3-"* ]]; then
    [[ "$labels" == *"priority:critical"* ]] && add+=("P0-critical")
    [[ "$labels" == *"priority:high"* ]] && add+=("P1-high")
    [[ "$labels" == *"priority:medium"* ]] && add+=("P2-medium")
    [[ "$labels" == *"priority:low"* ]] && add+=("P3-low")
  fi

  # area: map legacy unprefixed to prefixed
  if [[ "$labels" != *"area:"* ]]; then
    [[ "$labels" == *"parser"* ]] && add+=("area:parser")
    [[ "$labels" == *"lsp"* ]] && add+=("area:lsp")
    [[ "$labels" == *"tests"* ]] && add+=("area:tests")
    [[ "$labels" == *"infrastructure"* ]] && add+=("area:ci")
    [[ "$labels" == *"dap"* ]] && add+=("area:dap")
  fi

  # status: map legacy to prefixed, default to needs-triage
  if [[ "$labels" != *"status:"* ]]; then
    [[ "$labels" == *"blocked"* ]] && add+=("status:blocked")
    [[ "$labels" == *"in-progress"* ]] && add+=("status:in-progress")
    # If no status indicators, mark as needs-triage
    [[ "$labels" != *"blocked"* && "$labels" != *"in-progress"* ]] && add+=("status:needs-triage")
  fi

  if (( ${#add[@]} > 0 )); then
    joined="$(IFS=,; echo "${add[*]}")"
    if (( apply )); then
      gh issue edit "$num" --add-label "$joined" >/dev/null
      echo "✔ #$num +$joined"
    else
      echo "gh issue edit $num --add-label \"$joined\"  # $title"
    fi
  fi
done

if (( ! apply )); then
  echo ""
  echo "💡 To apply these changes, run: bash scripts/gh/backfill-prefixed-labels.sh --apply"
fi

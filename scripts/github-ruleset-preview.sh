#!/usr/bin/env bash
set -euo pipefail

# Read-only ruleset preview for repository maintainers.
#
# Usage:
#   scripts/github-ruleset-preview.sh [owner/repo]
#
# Behavior:
# - Queries existing repository rulesets.
# - Prints whether required_status_checks rules are present.
# - Never mutates settings.

repo_arg="${1:-}"

if ! command -v gh >/dev/null 2>&1; then
  echo "error: GitHub CLI (gh) is not installed." >&2
  exit 1
fi

if [[ -n "$repo_arg" ]]; then
  repo="$repo_arg"
else
  repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)"
fi

if [[ -z "$repo" ]]; then
  echo "error: unable to determine repository. Pass owner/repo explicitly." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  cat >&2 <<MSG
warning: gh is not authenticated.
Run: gh auth login
Then rerun: scripts/github-ruleset-preview.sh $repo
MSG
  exit 2
fi

endpoint="repos/$repo/rulesets"

if ! json="$(gh api -H 'Accept: application/vnd.github+json' "$endpoint" 2>/dev/null)"; then
  cat >&2 <<MSG
warning: unable to read $endpoint.
This is read-only and commonly fails when the token lacks repository admin/ruleset read permissions.
MSG
  exit 3
fi

if [[ "$(gh api -H 'Accept: application/vnd.github+json' "$endpoint" --jq 'length')" -eq 0 ]]; then
  echo "No rulesets found for $repo."
  exit 0
fi

echo "Repository: $repo"
echo "Endpoint: $endpoint"
echo

gh api -H 'Accept: application/vnd.github+json' "$endpoint" --jq '
  .[] |
  {
    id,
    name,
    target,
    enforcement,
    required_status_checks: ([.rules[]? | select(.type == "required_status_checks")] | length)
  } |
  "ruleset \(.id): \(.name) | target=\(.target) | enforcement=\(.enforcement) | required_status_checks_rules=\(.required_status_checks)"
'

echo
echo "Required status check details (if configured):"
gh api -H 'Accept: application/vnd.github+json' "$endpoint" --jq '
  .[] as $rs |
  ($rs.rules[]? | select(.type == "required_status_checks")) as $rule |
  "- \($rs.name) (id=\($rs.id)): required checks => \(($rule.parameters.required_status_checks // []) | map(.context) | join(", "))"
' || true

echo
echo "Read-only preview complete. No repository settings were changed."

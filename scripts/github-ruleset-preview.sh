#!/usr/bin/env bash
set -euo pipefail

# Read-only Ruleset preview helper.
# - Queries current repository rulesets.
# - Prints whether required_status_checks appears in each ruleset.
# - Never mutates repository settings.

REPO="${GITHUB_REPOSITORY:-}"
if [[ -z "${REPO}" ]]; then
  if git remote get-url origin >/dev/null 2>&1; then
    ORIGIN_URL="$(git remote get-url origin)"
    if [[ "${ORIGIN_URL}" =~ github.com[:/]([^/]+/[^/.]+)(\.git)?$ ]]; then
      REPO="${BASH_REMATCH[1]}"
    fi
  fi
fi

if [[ -z "${REPO}" ]]; then
  cat <<'MSG'
Unable to determine repository.
Set GITHUB_REPOSITORY=owner/repo and re-run.
MSG
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  cat <<'MSG'
GitHub CLI (gh) is not installed.
Install gh, authenticate, and re-run for a read-only preview.
MSG
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  cat <<'MSG'
GitHub CLI is not authenticated.
Run: gh auth login
Required scope: read access to repository rulesets.
MSG
  exit 1
fi

echo "Repository: ${REPO}"
echo "Fetching rulesets (read-only)..."

RULESETS_JSON="$(gh api -H 'Accept: application/vnd.github+json' "/repos/${REPO}/rulesets")"
COUNT="$(jq 'length' <<<"${RULESETS_JSON}")"

echo "Rulesets found: ${COUNT}"

if [[ "${COUNT}" -eq 0 ]]; then
  echo "No rulesets configured."
  exit 0
fi

echo
jq -r '
  .[] |
  . as $r |
  [
    "- name: \($r.name)",
    "  id: \($r.id)",
    "  target: \($r.target)",
    "  enforcement: \($r.enforcement)",
    "  required_status_checks: " +
      (if any($r.rules[]?; .type == "required_status_checks") then "present" else "absent" end)
  ] | join("\n")
' <<<"${RULESETS_JSON}"

echo
echo "Details (required_status_checks rules only):"
jq -r '
  .[] as $r |
  ($r.rules[]? | select(.type == "required_status_checks")) as $rule |
  "- \($r.name) [id=\($r.id)]\n" +
  (if ($rule.parameters.required_status_checks // []) | length == 0
   then "  checks: (none listed)"
   else (
     "  checks:\n" +
     (($rule.parameters.required_status_checks // [])
       | map("    - " + (.context // "<missing-context>"))
       | join("\n"))
   )
  )
' <<<"${RULESETS_JSON}" || true

echo
echo "Note: this script is read-only. It does not PATCH/PUT or modify rulesets."

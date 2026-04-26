#!/usr/bin/env bash
set -euo pipefail

# Read-only GitHub Ruleset preview for the current repository.
# This script intentionally does not mutate settings.
# It performs GET requests only.

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

infer_repo() {
  if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    printf '%s\n' "$GITHUB_REPOSITORY"
    return 0
  fi

  local remote
  remote="$(git remote get-url origin 2>/dev/null || true)"

  if [[ -z "$remote" ]]; then
    return 1
  fi

  # git@github.com:owner/repo.git
  # https://github.com/owner/repo.git
  if [[ "$remote" =~ github\.com[:/]([^/]+)/([^/.]+)(\.git)?$ ]]; then
    printf '%s/%s\n' "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
    return 0
  fi

  return 1
}

if ! has_cmd gh; then
  cat <<'MSG'
warning: gh CLI is not installed.
Install gh, then rerun this script to preview rulesets.
No changes were applied.
MSG
  exit 0
fi

if ! has_cmd jq; then
  cat <<'MSG'
warning: jq is not installed.
Install jq, then rerun this script to preview rulesets.
No changes were applied.
MSG
  exit 0
fi

if ! has_cmd git; then
  cat <<'MSG'
warning: git is not installed or unavailable in PATH.
No changes were applied.
MSG
  exit 0
fi

if ! gh auth status >/dev/null 2>&1; then
  cat <<'MSG'
warning: gh is not authenticated for API access.
Run 'gh auth login' (or set GH_TOKEN/GITHUB_TOKEN with repo read permissions)
then rerun this script.
No changes were applied.
MSG
  exit 0
fi

repo="$(infer_repo || true)"
if [[ -z "$repo" ]]; then
  echo "error: unable to determine repository (set GITHUB_REPOSITORY=owner/name)" >&2
  exit 1
fi

owner="${repo%%/*}"
name="${repo##*/}"

printf 'Repository: %s\n\n' "$repo"

echo '=== Rulesets (read-only preview) ==='
if ! rulesets_json="$(gh api "repos/${owner}/${name}/rulesets" 2>/dev/null)"; then
  cat <<'MSG'
warning: unable to read rulesets.
Ensure token has access to repository rulesets (repo administration read scope).
No changes were applied.
MSG
  exit 0
fi

if [[ "$(jq 'length' <<<"$rulesets_json")" -eq 0 ]]; then
  echo 'No rulesets found.'
  echo 'No changes were applied.'
  exit 0
fi

jq -r '
  .[] |
  [
    "- id: " + (.id|tostring),
    "  name: " + .name,
    "  target: " + (.target // "<none>"),
    "  enforcement: " + (.enforcement // "<none>"),
    "  bypass_actors: " + ((.bypass_actors|length|tostring) // "0")
  ] | .[]
' <<<"$rulesets_json"

echo
echo '=== Required status checks by ruleset ==='
jq -r '
  .[] as $rs |
  ($rs.rules // [])
  | map(select(.type == "required_status_checks"))
  | if length == 0 then
      "- " + $rs.name + ": <none>"
    else
      "- " + $rs.name + ":",
      (.[].parameters.required_status_checks[]?.context // "<missing-context>" | "    - " + .)
    end
' <<<"$rulesets_json"

echo
echo 'Done. No changes were applied.'

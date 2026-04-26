#!/usr/bin/env bash
set -euo pipefail

# Read-only preview of GitHub rulesets for the current repository.
# This script intentionally performs GET requests only.

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

infer_repo() {
  local origin_url owner_repo
  origin_url="$(git remote get-url origin 2>/dev/null || true)"
  if [[ -z "${origin_url}" ]]; then
    return 1
  fi

  # Supports:
  # - git@github.com:owner/repo.git
  # - https://github.com/owner/repo.git
  # - https://github.com/owner/repo
  if [[ "${origin_url}" =~ github\.com[:/]([^/]+/[^/.]+)(\.git)?$ ]]; then
    owner_repo="${BASH_REMATCH[1]}"
    printf '%s\n' "${owner_repo}"
    return 0
  fi

  return 1
}

require_cmd git
require_cmd gh
require_cmd jq

repo="${1:-}"
if [[ -z "${repo}" ]]; then
  if ! repo="$(infer_repo)"; then
    echo "error: unable to infer owner/repo from git remote; pass it explicitly:"
    echo "  scripts/github-ruleset-preview.sh owner/repo"
    exit 1
  fi
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "warning: gh is not authenticated. Run 'gh auth login' or export GITHUB_TOKEN."
  echo "read-only preview could not proceed."
  exit 0
fi

echo "Repository: ${repo}"
echo

echo "=== Repository rulesets (read-only GET) ==="
if ! gh api "repos/${repo}/rulesets" >/tmp/rulesets_repo.json 2>/tmp/rulesets_repo.err; then
  echo "warning: failed to read repository rulesets."
  cat /tmp/rulesets_repo.err
  exit 0
fi

jq -r '
  if length == 0 then
    "(none)"
  else
    .[]
    | [
        "- id=" + (.id|tostring),
        "name=" + .name,
        "target=" + .target,
        "enforcement=" + .enforcement
      ]
    | join(" ")
  end
' /tmp/rulesets_repo.json

echo
echo "=== required_status_checks details (if present) ==="
jq -r '
  .[]
  | . as $rs
  | ($rs.rules // [])[]?
  | select(.type == "required_status_checks")
  | "Ruleset: " + $rs.name,
    (
      (.parameters.required_status_checks // [])
      | if length == 0 then "  checks: (none configured)"
        else .[] | "  - " + (.context // "<missing-context>")
        end
    )
' /tmp/rulesets_repo.json

echo
echo "=== Branch protection summary (legacy endpoint, read-only GET) ==="
if gh api "repos/${repo}/branches/master/protection" >/tmp/master_protection.json 2>/tmp/master_protection.err; then
  jq -r '
    [
      "master.protected=true",
      "required_status_checks.strict=" + ((.required_status_checks.strict // false)|tostring),
      "required_status_checks.count=" + (((.required_status_checks.contexts // [])|length)|tostring)
    ]
    | join(" ")
  ' /tmp/master_protection.json
else
  echo "warning: unable to read master branch protection (branch may be absent or caller lacks permission)."
fi

echo
echo "Done. No settings were changed."

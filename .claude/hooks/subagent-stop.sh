#!/usr/bin/env bash
set -euo pipefail

OPS_DIR="${OPS_DIR:-.ops-perl-lsp}"
METRICS_FILE="${OPS_DIR}/swarm-metrics.jsonl"

mkdir -p "${OPS_DIR}"

INPUT="$(cat)"

payload_field() {
  local query="$1"
  echo "${INPUT}" | jq -r "${query}" | tr -d '\r'
}

TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
AGENT_NAME="$(payload_field '.subagent_name // .agent_name // .teammate_name // "unknown"')"
AGENT_TYPE="$(payload_field '.subagent_type // .agent_type // .matcher // "unknown"')"
# Prefer cwd (platform-provided) over worktree_path (not in platform payload)
WORKTREE_PATH="$(payload_field '.cwd // .worktree_path // .path // empty')"
SESSION_ID="$(payload_field '.session_id // empty')"

jq -nc \
  --arg ts "${TIMESTAMP}" \
  --arg event "subagent_stop" \
  --arg agent_name "${AGENT_NAME}" \
  --arg agent_type "${AGENT_TYPE}" \
  --arg worktree_path "${WORKTREE_PATH}" \
  --arg session_id "${SESSION_ID}" \
  '{ts:$ts,event:$event,agent_name:$agent_name,agent_type:$agent_type,worktree_path:$worktree_path,session_id:$session_id}' >> "${METRICS_FILE}"

resolve_plan_review_issue_num() {
  local resolved="${ISSUE_NUMBER:-}"
  if [[ -n "${resolved}" ]]; then
    printf '%s\n' "${resolved}"
    return
  fi

  resolved="$(payload_field '.issue_number // empty' 2>/dev/null || true)"
  if [[ -n "${resolved}" ]]; then
    printf '%s\n' "${resolved}"
    return
  fi

  if [[ "${AGENT_NAME}" =~ ^plan-review-([1-9][0-9]*)$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return
  fi

  if [[ "${AGENT_NAME}" =~ ^plan-reviewer-([1-9][0-9]*)$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
  fi
}

# -- Plan-reviewer label gate -----------------------------------------------
# When a plan-reviewer agent stops, verify they added a terminal label
# (builder-ready or already-fixed) to the issue they reviewed.
# This enforces the "never punt" rule from CLAUDE.md.
#
# Issue number resolution (in priority order):
#   1. $ISSUE_NUMBER environment variable (explicit, preferred)
#   2. issue_number field in the stdin JSON payload
#   3. canonical plan-review-NNN / plan-reviewer-NNN agent name
#
# The hook intentionally does NOT derive the issue number from the branch
# name. Plan-reviewers run in generic worktree slots named
# worktree-agent-<8hexchars>. Extracting digits from such a name produces
# garbage (e.g., branch "worktree-agent-a071b609" -> "71", not the actual
# issue). Any branch-name digit scan is banned here.
#
# Guards:
# - Only runs for plan-reviewer agent type
# - Requires a valid positive-integer issue number from env, payload, or name
# - If no valid issue number is available, fails loud (exit 3) rather than
#   silently labeling a random issue/PR
# - Accepts builder-ready OR already-fixed as valid terminal states
if [[ "${AGENT_TYPE}" == *"plan-reviewer"* ]]; then
  RESOLVED_ISSUE_NUM="$(resolve_plan_review_issue_num)"

  # Validate: must be a non-empty positive integer
  if [[ -z "${RESOLVED_ISSUE_NUM}" ]] || ! [[ "${RESOLVED_ISSUE_NUM}" =~ ^[1-9][0-9]*$ ]]; then
    echo "subagent-stop: plan-reviewer completed but no valid ISSUE_NUMBER is set." >&2
    echo "  Set ISSUE_NUMBER=<n> (positive integer), include issue_number in the" >&2
    echo "  agent JSON payload, or use the canonical plan-review-NNN agent name." >&2
    echo "  Branch-name digit extraction is banned -- it produces garbage for generic" >&2
    echo "  worktree slot names like worktree-agent-<8hexchars>." >&2
    exit 3
  fi

  LABELS="$(gh issue view "${RESOLVED_ISSUE_NUM}" --json labels --jq '[.labels[].name] | join(",")' 2>/dev/null || true)"
  if [[ -n "${LABELS}" ]]; then
    if [[ "${LABELS}" != *"builder-ready"* ]] && [[ "${LABELS}" != *"already-fixed"* ]]; then
      echo "Plan review incomplete: issue #${RESOLVED_ISSUE_NUM} does not have builder-ready or already-fixed label." >&2
      echo "Add the label before completing: gh issue edit ${RESOLVED_ISSUE_NUM} --add-label builder-ready" >&2
      exit 2
    fi
  fi
fi

exit 0

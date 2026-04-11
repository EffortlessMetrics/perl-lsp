#!/usr/bin/env bash
set -euo pipefail

OPS_DIR="${OPS_DIR:-.ops-perl-lsp}"
METRICS_FILE="${OPS_DIR}/swarm-metrics.jsonl"

mkdir -p "${OPS_DIR}"

INPUT="$(cat)"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
AGENT_NAME="$(echo "${INPUT}" | jq -r '.subagent_name // .agent_name // .teammate_name // "unknown"')"
AGENT_TYPE="$(echo "${INPUT}" | jq -r '.subagent_type // .agent_type // .matcher // "unknown"')"
# Prefer cwd (platform-provided) over worktree_path (not in platform payload)
WORKTREE_PATH="$(echo "${INPUT}" | jq -r '.cwd // .worktree_path // .path // empty')"
SESSION_ID="$(echo "${INPUT}" | jq -r '.session_id // empty')"

jq -nc   --arg ts "${TIMESTAMP}"   --arg event "subagent_stop"   --arg agent_name "${AGENT_NAME}"   --arg agent_type "${AGENT_TYPE}"   --arg worktree_path "${WORKTREE_PATH}"   --arg session_id "${SESSION_ID}"   '{ts:$ts,event:$event,agent_name:$agent_name,agent_type:$agent_type,worktree_path:$worktree_path,session_id:$session_id}' >> "${METRICS_FILE}"

# -- Plan-reviewer label gate -----------------------------------------------
# When a plan-reviewer agent stops, verify they added a terminal label
# (builder-ready or already-fixed) to the issue they reviewed.
# This enforces the "never punt" rule from CLAUDE.md.
#
# Issue number resolution (in priority order):
#   1. $ISSUE_NUMBER environment variable (explicit, preferred)
#   2. issue_number field in the stdin JSON payload
#
# The hook intentionally does NOT derive the issue number from the branch
# name. Plan-reviewers run in generic worktree slots named
# worktree-agent-<8hexchars>. Extracting digits from such a name produces
# garbage (e.g., branch "worktree-agent-a071b609" -> "71", not the actual
# issue). Any branch-name digit scan is banned here.
#
# Guards:
# - Only runs for plan-reviewer agent type
# - Requires a valid positive-integer ISSUE_NUMBER (env var or JSON field)
# - If no valid issue number is available, fails loud (exit 3) rather than
#   silently labeling a random issue/PR
# - Accepts builder-ready OR already-fixed as valid terminal states
if [[ "${AGENT_TYPE}" == *"plan-reviewer"* ]]; then
  # Resolve issue number: env var takes priority, then JSON payload field
  RESOLVED_ISSUE_NUM="${ISSUE_NUMBER:-}"
  if [[ -z "${RESOLVED_ISSUE_NUM}" ]]; then
    RESOLVED_ISSUE_NUM="$(echo "${INPUT}" | jq -r '.issue_number // empty' 2>/dev/null || true)"
  fi

  # Validate: must be a non-empty positive integer
  if [[ -z "${RESOLVED_ISSUE_NUM}" ]] || ! [[ "${RESOLVED_ISSUE_NUM}" =~ ^[1-9][0-9]*$ ]]; then
    echo "subagent-stop: plan-reviewer completed but no valid ISSUE_NUMBER is set." >&2
    echo "  Set ISSUE_NUMBER=<n> (positive integer) before the agent runs, or include" >&2
    echo "  issue_number in the agent's JSON payload." >&2
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

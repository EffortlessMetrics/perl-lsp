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

jq -nc \
  --arg ts "${TIMESTAMP}" \
  --arg event "subagent_stop" \
  --arg agent_name "${AGENT_NAME}" \
  --arg agent_type "${AGENT_TYPE}" \
  --arg worktree_path "${WORKTREE_PATH}" \
  --arg session_id "${SESSION_ID}" \
  '{ts:$ts,event:$event,agent_name:$agent_name,agent_type:$agent_type,worktree_path:$worktree_path,session_id:$session_id}' >> "${METRICS_FILE}"

# ── Plan-reviewer label gate ──────────────────────────────────────────────
# When a plan-reviewer agent stops, verify they added a terminal label
# (builder-ready or already-fixed) to the issue they reviewed.
# This enforces the "never punt" rule from CLAUDE.md.
#
# Guards:
# - Only runs for plan-reviewer agent type
# - Requires WORKTREE_PATH to extract branch/issue number
# - Skips gracefully on any infra failure (gh unavailable, no issue num, etc)
# - Accepts builder-ready OR already-fixed as valid terminal states
if [[ "${AGENT_TYPE}" == *"plan-reviewer"* ]] && [[ -n "${WORKTREE_PATH}" ]]; then
  BRANCH="$(git -C "${WORKTREE_PATH}" branch --show-current 2>/dev/null || true)"
  ISSUE_NUM="$(echo "${BRANCH}" | grep -oE '[0-9]+' | head -1 || true)"
  if [[ -n "${ISSUE_NUM}" ]]; then
    LABELS="$(gh issue view "${ISSUE_NUM}" --json labels --jq '[.labels[].name] | join(",")' 2>/dev/null || true)"
    if [[ -n "${LABELS}" ]]; then
      if [[ "${LABELS}" != *"builder-ready"* ]] && [[ "${LABELS}" != *"already-fixed"* ]]; then
        echo "Plan review incomplete: issue #${ISSUE_NUM} does not have builder-ready or already-fixed label." >&2
        echo "Add the label before completing: gh issue edit ${ISSUE_NUM} --add-label builder-ready" >&2
        exit 2
      fi
    fi
  fi
fi

exit 0

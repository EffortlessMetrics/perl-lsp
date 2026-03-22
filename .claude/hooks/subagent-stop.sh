#!/usr/bin/env bash
set -euo pipefail

OPS_DIR="${OPS_DIR:-.ops-perl-lsp}"
METRICS_FILE="${OPS_DIR}/swarm-metrics.jsonl"

mkdir -p "${OPS_DIR}"

INPUT="$(cat)"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
AGENT_NAME="$(echo "${INPUT}" | jq -r '.subagent_name // .agent_name // .teammate_name // "unknown"')"
AGENT_TYPE="$(echo "${INPUT}" | jq -r '.subagent_type // .agent_type // .matcher // "unknown"')"
WORKTREE_PATH="$(echo "${INPUT}" | jq -r '.worktree_path // .path // .tool_input.worktree_path // empty')"
SESSION_ID="$(echo "${INPUT}" | jq -r '.session_id // empty')"

jq -nc \
  --arg ts "${TIMESTAMP}" \
  --arg event "subagent_stop" \
  --arg agent_name "${AGENT_NAME}" \
  --arg agent_type "${AGENT_TYPE}" \
  --arg worktree_path "${WORKTREE_PATH}" \
  --arg session_id "${SESSION_ID}" \
  '{ts:$ts,event:$event,agent_name:$agent_name,agent_type:$agent_type,worktree_path:$worktree_path,session_id:$session_id}' >> "${METRICS_FILE}"

# ── Plan-reviewer label gate ──────────────────────────────────────────────────
# When a plan-reviewer subagent stops, verify that builder-ready or already-fixed
# was applied. Uses worktree path to extract branch → issue number.
# Fails open on any infrastructure error (missing gh, no network, unparseable path).

if [[ "${AGENT_TYPE}" == *"plan-reviewer"* && -n "${WORKTREE_PATH}" && -d "${WORKTREE_PATH}" ]]; then
    BRANCH="$(git -C "${WORKTREE_PATH}" branch --show-current 2>/dev/null || true)"
    ISSUE_NUM="$(echo "${BRANCH}" | grep -oE '[0-9]+' | head -1 || true)"
    if [[ -n "${ISSUE_NUM}" ]]; then
        LABELS="$(gh issue view "${ISSUE_NUM}" --json labels --jq '.labels[].name' 2>/dev/null || true)"
        if [[ -n "${LABELS}" ]] && ! echo "${LABELS}" | grep -qE '^(builder-ready|already-fixed)$'; then
            echo "Plan review incomplete: issue #${ISSUE_NUM} missing builder-ready or already-fixed label." >&2
            echo "Fix: gh issue edit ${ISSUE_NUM} --add-label builder-ready --add-label plan-reviewed --remove-label needs-plan-review" >&2
            exit 2
        fi
    fi
fi

exit 0

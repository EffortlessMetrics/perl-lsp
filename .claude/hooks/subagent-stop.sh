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

exit 0

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

exit 0

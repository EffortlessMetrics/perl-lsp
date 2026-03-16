#!/usr/bin/env bash
set -euo pipefail

OPS_DIR="${OPS_DIR:-.ops}"
METRICS_FILE="${OPS_DIR}/swarm-metrics.jsonl"

mkdir -p "${OPS_DIR}"

INPUT="$(cat)"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
WORKTREE_PATH="$(echo "${INPUT}" | jq -r '.worktree_path // .path // .target_path // empty')"
BRANCH_NAME="$(echo "${INPUT}" | jq -r '.branch // .branch_name // .head_ref // empty')"
SESSION_ID="$(echo "${INPUT}" | jq -r '.session_id // empty')"

jq -nc \
  --arg ts "${TIMESTAMP}" \
  --arg event "worktree_remove" \
  --arg worktree_path "${WORKTREE_PATH}" \
  --arg branch "${BRANCH_NAME}" \
  --arg session_id "${SESSION_ID}" \
  '{ts:$ts,event:$event,worktree_path:$worktree_path,branch:$branch,session_id:$session_id}' >> "${METRICS_FILE}"

exit 0

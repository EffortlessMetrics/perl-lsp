#!/bin/bash
# TaskCompleted hook: verify quality before allowing task completion
# Exit 2 = reject completion with feedback
# Exit 0 = allow completion

# Read stdin once at the top -- stdin can only be consumed once, so capture before any subshells.
# Hook tests may invoke this script without piped input; avoid blocking forever on an open stdin.
INPUT='{}'
if [[ ! -t 0 ]]; then
  FIRST_CHAR=''
  if IFS= read -r -t 1 -n 1 FIRST_CHAR 2>/dev/null; then
    # Once the first byte arrives, consume the rest of the payload.
    INPUT="${FIRST_CHAR}$(cat 2>/dev/null || true)"
    [[ -z "${INPUT}" ]] && INPUT='{}'
  fi
fi

payload_field() {
  local query="$1"
  echo "${INPUT}" | jq -r "${query}" 2>/dev/null | tr -d '\r'
}

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"

# Quick sanity check: is cargo fmt clean?
# Guard: only run cargo fmt if the agent staged or recently committed .rs files.
# Guard HEAD~1: on first commit (shallow clone, fresh repo), check if any tracked .rs files exist.
HAS_RS_DIFF=0
if git diff --cached --name-only 2>/dev/null | grep -q '\.rs$'; then
  HAS_RS_DIFF=1
elif git rev-parse HEAD~1 &>/dev/null 2>&1 && git diff --name-only HEAD~1 2>/dev/null | grep -q '\.rs$'; then
  HAS_RS_DIFF=1
elif ! git rev-parse HEAD~1 &>/dev/null 2>&1 && git ls-files -- '*.rs' 2>/dev/null | grep -q .; then
  HAS_RS_DIFF=1
fi

if [[ "${HAS_RS_DIFF}" -eq 1 ]]; then
  if ! cargo xtask fmt --check 2>/dev/null; then
    echo "Task completion blocked: cargo fmt check failed. Run 'cargo xtask fmt' before marking complete."
    exit 2
  fi
fi

# Check if test files were modified and CURRENT_STATUS.md needs updating
HAS_TEST_DIFF=0
if git diff --cached --name-only 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$'; then
  HAS_TEST_DIFF=1
elif git rev-parse HEAD~1 &>/dev/null 2>&1 && git diff --name-only HEAD~1 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$'; then
  HAS_TEST_DIFF=1
elif ! git rev-parse HEAD~1 &>/dev/null 2>&1 && git ls-files -- 'crates/*/tests/*.rs' 2>/dev/null | grep -q .; then
  HAS_TEST_DIFF=1
fi

if [[ "${HAS_TEST_DIFF}" -eq 1 ]]; then
  if command -v python3 &>/dev/null && [ -f "$REPO_ROOT/scripts/update-current-status.py" ]; then
    python3 "$REPO_ROOT/scripts/update-current-status.py" 2>/dev/null || true
    if ! git diff --quiet -- docs/project/CURRENT_STATUS.md 2>/dev/null; then
      echo "Task completion blocked: test files changed but CURRENT_STATUS.md has stale counts."
      echo "Run: python3 scripts/update-current-status.py && git add docs/project/CURRENT_STATUS.md"
      exit 2
    fi
  fi
fi

# Passive metrics write: capture task completion event into swarm-metrics.jsonl
# This is advisory (exit 0 always) -- lifecycle ordering prevents a blocking gate here.
# SubagentStop fires AFTER TaskCompleted, so session-correlated matching is impossible at this point.
# See: https://github.com/EffortlessMetrics/perl-lsp/issues/2811
OPS_DIR="${OPS_DIR:-${REPO_ROOT}/.ops-perl-lsp}"
METRICS_FILE="${OPS_DIR}/swarm-metrics.jsonl"

if command -v jq &>/dev/null; then
  SESSION_ID="$(payload_field '.session_id // empty' || echo '')"
  CWD="$(payload_field '.cwd // empty' || echo '')"
  TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  mkdir -p "${OPS_DIR}" 2>/dev/null || true
  jq -nc \
    --arg ts "${TIMESTAMP}" \
    --arg event "task_completed" \
    --arg session_id "${SESSION_ID}" \
    --arg cwd "${CWD}" \
    '{ts:$ts,event:$event,session_id:$session_id,cwd:$cwd}' >> "${METRICS_FILE}" 2>/dev/null || true
fi

# Allow completion
exit 0

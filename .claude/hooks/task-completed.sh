#!/bin/bash
# TaskCompleted hook: verify quality before allowing task completion
# Exit 2 = reject completion with feedback
# Exit 0 = allow completion

# Read stdin once at the top -- stdin can only be consumed once, so capture before any subshells.
# Hook tests may invoke this script without piped input; avoid blocking forever on an open stdin.
INPUT='{}'
if read -t 0 2>/dev/null; then
  # Input is available; consume it all at once.
  INPUT="$(cat 2>/dev/null)" || INPUT='{}'
  [[ -z "${INPUT}" ]] && INPUT='{}'
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"

# Quick sanity check: is cargo fmt clean?
# Guard: only run cargo fmt if the agent staged or recently committed .rs files.
# Note: git ls-files checks ALL tracked files (always matches in this repo), not modified files.
# We use git diff to detect actual .rs work by this agent -- same pattern as the test-file check below.
if git diff --cached --name-only 2>/dev/null | grep -q '\.rs$' || \
   git diff --name-only HEAD~1 2>/dev/null | grep -q '\.rs$'; then
  if ! cargo fmt --all -- --check 2>/dev/null; then
    echo "Task completion blocked: cargo fmt check failed. Run 'cargo fmt --all' before marking complete."
    exit 2
  fi
fi

# ── Receipt gate: verify-build, clippy, test ──────────────────────────────
# Receipt files are JSON blobs written by verify-build / clippy / test steps.
#
# Receipt format (path convention):
#   receipts/verify-build.<commit-hash>
#   receipts/clippy.<commit-hash>
#   receipts/test.<commit-hash>
#
# Content (JSON):
#   {"exit_code": 0, "timestamp": "<ISO 8601 UTC>", "duration_s": <float>}
#
# Staleness: receipts older than 1 hour are rejected (re-run verification).
# This gate only fires when .rs files are in the current diff.

RECEIPT_DIR="${REPO_ROOT}/receipts"
COMMIT_HASH="$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
MAX_AGE_SECONDS=3600  # 1 hour

if git diff --cached --name-only 2>/dev/null | grep -q '\.rs$' || \
   git diff --name-only HEAD~1 2>/dev/null | grep -q '\.rs$'; then

  MISSING=()
  STALE=()
  FAILED=()

  for CHECK in verify-build clippy test; do
    RECEIPT="${RECEIPT_DIR}/${CHECK}.${COMMIT_HASH}"

    if [[ ! -f "${RECEIPT}" ]]; then
      MISSING+=("${CHECK}")
      continue
    fi

    # Parse receipt fields
    if command -v jq &>/dev/null; then
      EXIT_CODE="$(jq -r '.exit_code // 1' "${RECEIPT}" 2>/dev/null)"
      TS_STR="$(jq -r '.timestamp // empty' "${RECEIPT}" 2>/dev/null)"
    else
      # Fallback: just check file is non-empty
      EXIT_CODE=0
      TS_STR=""
    fi

    if [[ "${EXIT_CODE}" != "0" ]]; then
      FAILED+=("${CHECK} (exit code ${EXIT_CODE})")
    elif [[ -n "${TS_STR}" ]]; then
      # Validate freshness
      TS_EPOCH="$(date -d "${TS_STR}" -u +%s 2>/dev/null || echo 0)"
      NOW_EPOCH="$(date -u +%s)"
      AGE=$(( NOW_EPOCH - TS_EPOCH ))
      if [[ "${AGE}" -gt "${MAX_AGE_SECONDS}" ]]; then
        STALE+=("${CHECK} (${AGE}s old, max ${MAX_AGE_SECONDS}s)")
      fi
    fi
  done

  BLOCKED=0
  if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "Task completion blocked: missing receipts: ${MISSING[*]}"
    BLOCKED=1
  fi
  if [[ ${#STALE[@]} -gt 0 ]]; then
    echo "Task completion blocked: stale receipts: ${STALE[*]}"
    BLOCKED=1
  fi
  if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo "Task completion blocked: failed receipts: ${FAILED[*]}"
    BLOCKED=1
  fi

  if [[ "${BLOCKED}" -eq 1 ]]; then
    echo ""
    echo "Run verify-build, clippy, and test steps to generate fresh receipts at:"
    echo "  receipts/{verify-build,clippy,test}.${COMMIT_HASH}"
    echo ""
    echo "Receipt format: {\"exit_code\": 0, \"timestamp\": \"<ISO 8601 UTC>\", \"duration_s\": <float>}"
    echo "Receipts must be less than ${MAX_AGE_SECONDS}s old."
    exit 2
  fi
fi

# Check if test files were modified and CURRENT_STATUS.md needs updating
if git diff --cached --name-only 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$' || \
   git diff --name-only HEAD~1 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$'; then
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
  SESSION_ID="$(echo "${INPUT}" | jq -r '.session_id // empty' 2>/dev/null || echo '')"
  CWD="$(echo "${INPUT}" | jq -r '.cwd // empty' 2>/dev/null || echo '')"
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

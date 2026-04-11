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

receipt_field() {
  local receipt="$1"
  local query="$2"
  jq -r "${query}" "${receipt}" 2>/dev/null | tr -d '\r'
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

# Fix #1: require jq — without it we cannot validate receipt JSON
if ! command -v jq &>/dev/null; then
  echo "Task completion blocked: jq is required for receipt validation but not found in PATH."
  echo "Install jq (e.g. apt install jq, brew install jq) and re-run."
  exit 2
fi

# Fix #5: no silent fallback — fail hard if git is unavailable
COMMIT_HASH="$(git rev-parse --short HEAD 2>/dev/null)" || {
  echo "Task completion blocked: could not determine current commit hash via git."
  exit 2
}

MAX_AGE_SECONDS=3600  # 1 hour

# Fix #3: guard HEAD~1 — on first commit (shallow clone, fresh repo) fall back to all tracked .rs files
RS_CHANGED=0
if git diff --cached --name-only 2>/dev/null | grep -q '\.rs$'; then
  RS_CHANGED=1
elif git rev-parse HEAD~1 &>/dev/null 2>&1 && git diff --name-only HEAD~1 2>/dev/null | grep -q '\.rs$'; then
  RS_CHANGED=1
elif ! git rev-parse HEAD~1 &>/dev/null 2>&1; then
  # No parent commit — check if any tracked .rs files exist (repo has Rust code)
  if git ls-files -- '*.rs' 2>/dev/null | grep -q .; then
    RS_CHANGED=1
  fi
fi

if [[ "${RS_CHANGED}" -eq 1 ]]; then

  MISSING=()
  STALE=()
  FAILED=()

  for CHECK in verify-build clippy test; do
    RECEIPT="${RECEIPT_DIR}/${CHECK}.${COMMIT_HASH}"

    if [[ ! -f "${RECEIPT}" ]]; then
      MISSING+=("${CHECK}")
      continue
    fi

    # Fix #6: validate receipt JSON structure before trusting fields
    EXIT_CODE="$(receipt_field "${RECEIPT}" '.exit_code // empty')"
    TS_STR="$(receipt_field "${RECEIPT}" '.timestamp // empty')"
    DURATION="$(receipt_field "${RECEIPT}" '.duration_s // empty')"

    # Structural validation: must have exit_code and timestamp, timestamp must match UTC pattern
    if ! jq -e '(.exit_code | type) == "number" and (.timestamp | type) == "string"' "${RECEIPT}" &>/dev/null; then
      FAILED+=("${CHECK} (malformed receipt: missing or invalid exit_code/timestamp)")
      continue
    fi

    # Fix #4: enforce UTC — timestamp must end with Z or +00:00
    if ! [[ "${TS_STR}" =~ Z$ || "${TS_STR}" =~ \+00:00$ ]]; then
      FAILED+=("${CHECK} (timestamp not UTC: ${TS_STR})")
      continue
    fi

    if [[ "${EXIT_CODE}" != "0" ]]; then
      FAILED+=("${CHECK} (exit code ${EXIT_CODE})")
    elif [[ -n "${TS_STR}" ]]; then
      # Fix #2: reject future timestamps
      TS_EPOCH="$(date -d "${TS_STR}" -u +%s 2>/dev/null)" || {
        FAILED+=("${CHECK} (unparseable timestamp: ${TS_STR})")
        continue
      }
      NOW_EPOCH="$(date -u +%s)"
      if [[ "${TS_EPOCH}" -gt "${NOW_EPOCH}" ]]; then
        FAILED+=("${CHECK} (future timestamp: ${TS_STR})")
        continue
      fi
      # Validate freshness
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
    echo "Receipt format: {\"exit_code\": 0, \"timestamp\": \"<ISO 8601 UTC ending in Z>\", \"duration_s\": <float>}"
    echo "Receipts must be less than ${MAX_AGE_SECONDS}s old."
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

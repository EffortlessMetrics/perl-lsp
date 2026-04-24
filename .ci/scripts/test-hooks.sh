#!/usr/bin/env bash
# Hook behavior tests — plain-bash unit tests, no bats required.
# Modeled on .ci/scripts/check-from-raw.sh
#
# Exit codes:
#   0 — all tests passed
#   1 — one or more tests failed

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
HOOKS_DIR="$REPO_ROOT/.claude/hooks"

PASS=0
FAIL=0

# ---------------------------------------------------------------------------
# Assertion helpers
# ---------------------------------------------------------------------------

assert_exit() {
  local expected="$1"; shift
  local desc="$1"; shift
  local actual
  actual=0
  "$@" || actual=$?
  if [[ "$actual" -eq "$expected" ]]; then
    echo "  PASS: $desc (exit $actual)"
    PASS=$(( PASS + 1 ))
  else
    echo "  FAIL: $desc — expected exit $expected, got $actual" >&2
    FAIL=$(( FAIL + 1 ))
  fi
}

assert_file_exists() {
  local path="$1"
  local desc="$2"
  if [[ -f "$path" ]]; then
    echo "  PASS: $desc"
    PASS=$(( PASS + 1 ))
  else
    echo "  FAIL: $desc — file not found: $path" >&2
    FAIL=$(( FAIL + 1 ))
  fi
}

assert_executable() {
  local path="$1"
  local desc="$2"
  if [[ -x "$path" ]]; then
    echo "  PASS: $desc"
    PASS=$(( PASS + 1 ))
  else
    echo "  FAIL: $desc — not executable: $path" >&2
    FAIL=$(( FAIL + 1 ))
  fi
}

assert_contains() {
  local content="$1"
  local pattern="$2"
  local desc="$3"
  if echo "$content" | grep -qE "$pattern"; then
    echo "  PASS: $desc"
    PASS=$(( PASS + 1 ))
  else
    echo "  FAIL: $desc — pattern '$pattern' not found in output" >&2
    FAIL=$(( FAIL + 1 ))
  fi
}

# ---------------------------------------------------------------------------
# Test group: hook files exist and are executable
# ---------------------------------------------------------------------------

echo ""
echo "=== Hook file existence and permissions ==="

assert_file_exists "$HOOKS_DIR/task-completed.sh"     "task-completed.sh exists"
assert_file_exists "$HOOKS_DIR/subagent-stop.sh"      "subagent-stop.sh exists"
assert_file_exists "$HOOKS_DIR/pre-tool-use.sh"       "pre-tool-use.sh exists"
assert_executable  "$HOOKS_DIR/task-completed.sh"     "task-completed.sh is executable"
assert_executable  "$HOOKS_DIR/subagent-stop.sh"      "subagent-stop.sh is executable"
assert_executable  "$HOOKS_DIR/pre-tool-use.sh"       "pre-tool-use.sh is executable"

# ---------------------------------------------------------------------------
# Test group: task-completed.sh
# ---------------------------------------------------------------------------

echo ""
echo "=== task-completed.sh behavior ==="

# task-completed.sh should pass with no .rs changes staged (clean environment)
assert_exit 0 "task-completed passes with no staged .rs files" \
  bash "$HOOKS_DIR/task-completed.sh"

# ---------------------------------------------------------------------------
# Test group: subagent-stop.sh writes JSONL
# ---------------------------------------------------------------------------

echo ""
echo "=== subagent-stop.sh behavior ==="

TMP_OPS="$(mktemp -d)"
trap 'rm -rf "$TMP_OPS"' EXIT

SAMPLE_PAYLOAD='{"subagent_name":"test-agent","subagent_type":"builder","session_id":"abc123"}'

OUTPUT_FILE="$TMP_OPS/swarm-metrics.jsonl"
echo "$SAMPLE_PAYLOAD" | OPS_DIR="$TMP_OPS" bash "$HOOKS_DIR/subagent-stop.sh"

assert_file_exists "$OUTPUT_FILE" "subagent-stop.sh creates swarm-metrics.jsonl"

if [[ -f "$OUTPUT_FILE" ]]; then
  LINE="$(cat "$OUTPUT_FILE")"
  assert_contains "$LINE" '"event":"subagent_stop"'  "JSONL contains event field"
  assert_contains "$LINE" '"agent_name":"test-agent"' "JSONL contains agent_name"
  assert_contains "$LINE" '"ts":"[0-9]{4}-'           "JSONL contains ts timestamp"
fi

# Custom OPS_DIR is respected
TMP_CUSTOM="$(mktemp -d)"
trap 'rm -rf "$TMP_OPS" "$TMP_CUSTOM"' EXIT
echo "$SAMPLE_PAYLOAD" | OPS_DIR="$TMP_CUSTOM" bash "$HOOKS_DIR/subagent-stop.sh"
assert_file_exists "$TMP_CUSTOM/swarm-metrics.jsonl" "subagent-stop.sh respects custom OPS_DIR"

# ---------------------------------------------------------------------------
# Test group: task-completed.sh -- metrics write
# ---------------------------------------------------------------------------

echo ""
echo "=== task-completed.sh metrics write ==="

TMP_OPS_TC="$(mktemp -d)"
SAMPLE_INPUT_TC='{"session_id":"abc123","cwd":"/repo/worktrees/agent-xyz"}'

assert_exit 0 "task-completed exits 0 with metrics payload"   bash -c "echo '${SAMPLE_INPUT_TC}' | OPS_DIR='${TMP_OPS_TC}' bash '${HOOKS_DIR}/task-completed.sh'"

assert_file_exists "${TMP_OPS_TC}/swarm-metrics.jsonl" "task-completed writes swarm-metrics.jsonl"

if [[ -f "${TMP_OPS_TC}/swarm-metrics.jsonl" ]]; then
  LINE_TC="$(cat "${TMP_OPS_TC}/swarm-metrics.jsonl")"
  assert_contains "${LINE_TC}" '"event":"task_completed"' "metrics entry has task_completed event"
  assert_contains "${LINE_TC}" '"session_id":"abc123"' "metrics entry captures session_id"
fi

TMP_OPS_TC2="$(mktemp -d)"
assert_exit 0 "task-completed exits 0 with empty payload"   bash -c "echo '{}' | OPS_DIR='${TMP_OPS_TC2}' bash '${HOOKS_DIR}/task-completed.sh'"

rm -rf "${TMP_OPS_TC}" "${TMP_OPS_TC2}" 2>/dev/null || true

# ---------------------------------------------------------------------------
# Test group: subagent-stop.sh -- cwd capture
# ---------------------------------------------------------------------------

echo ""
echo "=== subagent-stop.sh cwd capture ==="

TMP_OPS_SS="$(mktemp -d)"
CWD_PAYLOAD='{"subagent_type":"builder","cwd":"/repo/worktrees/agent-abc","session_id":"sess1"}'

echo "${CWD_PAYLOAD}" | OPS_DIR="${TMP_OPS_SS}" bash "${HOOKS_DIR}/subagent-stop.sh"

if [[ -f "${TMP_OPS_SS}/swarm-metrics.jsonl" ]]; then
  LINE_SS="$(tail -1 "${TMP_OPS_SS}/swarm-metrics.jsonl")"
  assert_contains "${LINE_SS}" '/repo/worktrees/agent-abc' "subagent-stop captures cwd as worktree_path"
fi

rm -rf "${TMP_OPS_SS}" 2>/dev/null || true


# ---------------------------------------------------------------------------
# Test group: subagent-stop.sh -- plan-reviewer issue number gate
# ---------------------------------------------------------------------------
# Regression tests for issue #4044: branch-name digit extraction produced
# garbage issue numbers for generic worktree slots (worktree-agent-<8hexchars>).
# The hook must now require explicit issue context: ISSUE_NUMBER env var,
# issue_number JSON field, or canonical plan-review-NNN agent name.
# ---------------------------------------------------------------------------

echo ""
echo "=== subagent-stop.sh plan-reviewer issue number gate (issue #4044) ==="

TMP_OPS_PR="$(mktemp -d)"

# Test 1: plan-reviewer with no ISSUE_NUMBER and no issue_number JSON field
# must exit 3 (fail loud) and NOT touch issue #71 (the bug: branch
# "worktree-agent-a071b609" used to extract "71" from the hex suffix)
PR_PAYLOAD_NO_NUM='{"subagent_name":"plan-reviewer","subagent_type":"plan-reviewer","cwd":"/repo/worktrees/agent-a071b609","session_id":"sess-pr"}'
assert_exit 3 "plan-reviewer without ISSUE_NUMBER exits 3 (fail loud, not silent)"   bash -c "echo '${PR_PAYLOAD_NO_NUM}' | OPS_DIR='${TMP_OPS_PR}' bash '${HOOKS_DIR}/subagent-stop.sh'"

# Test 2: plan-reviewer with ISSUE_NUMBER=0 (invalid, not a positive integer) exits 3
PR_PAYLOAD_ZERO='{"subagent_type":"plan-reviewer","session_id":"sess-pr2"}'
assert_exit 3 "plan-reviewer with ISSUE_NUMBER=0 exits 3 (must be positive integer)"   bash -c "echo '${PR_PAYLOAD_ZERO}' | OPS_DIR='${TMP_OPS_PR}' ISSUE_NUMBER=0 bash '${HOOKS_DIR}/subagent-stop.sh'"

# Test 3: plan-reviewer with ISSUE_NUMBER=abc (non-integer) exits 3
assert_exit 3 "plan-reviewer with ISSUE_NUMBER=abc exits 3 (must be integer)"   bash -c "echo '${PR_PAYLOAD_ZERO}' | OPS_DIR='${TMP_OPS_PR}' ISSUE_NUMBER=abc bash '${HOOKS_DIR}/subagent-stop.sh'"

# Test 4: non-plan-reviewer agent with hex-only branch name exits 0 (gate skipped)
BUILDER_PAYLOAD='{"subagent_type":"builder","cwd":"/repo/worktrees/agent-a071b609","session_id":"sess-builder"}'
assert_exit 0 "builder agent (non-plan-reviewer) with hex branch exits 0 (gate skipped)"   bash -c "echo '${BUILDER_PAYLOAD}' | OPS_DIR='${TMP_OPS_PR}' bash '${HOOKS_DIR}/subagent-stop.sh'"

# Test 5: JSONL metrics are still written even when the plan-review gate fails
PR_METRICS_FILE="${TMP_OPS_PR}/swarm-metrics.jsonl"
assert_file_exists "${PR_METRICS_FILE}" "Metrics file exists after plan-review gate failure"
assert_contains "$(tail -1 "${PR_METRICS_FILE}")" '"event":"subagent_stop"' "JSONL metrics written even when plan-review gate exits 3"

# Test 6: plan-reviewer with valid ISSUE_NUMBER via env var accepts the env var
# (gh call will fail in test env, but exit from gate is 0 or 2 -- not 3)
# We use a deliberately invalid issue number that gh won't find, so gh returns
# empty labels and the gate passes (no builder-ready check triggered).
PR_PAYLOAD_VALID='{"subagent_type":"plan-reviewer","session_id":"sess-pr3"}'
# gh will likely fail or return empty labels in test env -- gate should NOT exit 3
# We can't control gh output in tests, but we confirm exit is not 3
ACTUAL_EXIT=0
bash -c "echo '${PR_PAYLOAD_VALID}' | OPS_DIR='${TMP_OPS_PR}' ISSUE_NUMBER=99999999 bash '${HOOKS_DIR}/subagent-stop.sh'" || ACTUAL_EXIT=$?
if [[ "${ACTUAL_EXIT}" -ne 3 ]]; then
  echo "  PASS: plan-reviewer with valid ISSUE_NUMBER=99999999 does not exit 3 (got ${ACTUAL_EXIT})"
  PASS=$(( PASS + 1 ))
else
  echo "  FAIL: plan-reviewer with valid ISSUE_NUMBER=99999999 should not exit 3" >&2
  FAIL=$(( FAIL + 1 ))
fi

# Test 7: plan-reviewer with issue_number in JSON payload (fallback path)
# The env var takes priority; if absent, the hook reads issue_number from the JSON.
PR_PAYLOAD_JSON_NUM='{"subagent_type":"plan-reviewer","issue_number":99999998,"session_id":"sess-pr4"}'
ACTUAL_EXIT_JSON=0
bash -c "echo '${PR_PAYLOAD_JSON_NUM}' | OPS_DIR='${TMP_OPS_PR}' bash '${HOOKS_DIR}/subagent-stop.sh'" || ACTUAL_EXIT_JSON=$?
if [[ "${ACTUAL_EXIT_JSON}" -ne 3 ]]; then
  echo "  PASS: plan-reviewer with issue_number in JSON payload does not exit 3 (got ${ACTUAL_EXIT_JSON})"
  PASS=$(( PASS + 1 ))
else
  echo "  FAIL: plan-reviewer with issue_number in JSON payload should not exit 3" >&2
  FAIL=$(( FAIL + 1 ))
fi

# Test 8: canonical agent name plan-review-NNN is accepted as a fallback
PR_PAYLOAD_NAME_NUM='{"subagent_name":"plan-review-4044","subagent_type":"plan-reviewer","session_id":"sess-pr5"}'
ACTUAL_EXIT_NAME=0
bash -c "echo '${PR_PAYLOAD_NAME_NUM}' | OPS_DIR='${TMP_OPS_PR}' bash '${HOOKS_DIR}/subagent-stop.sh'" || ACTUAL_EXIT_NAME=$?
if [[ "${ACTUAL_EXIT_NAME}" -ne 3 ]]; then
  echo "  PASS: canonical plan-review-NNN agent name does not exit 3 (got ${ACTUAL_EXIT_NAME})"
  PASS=$(( PASS + 1 ))
else
  echo "  FAIL: canonical plan-review-NNN agent name should not exit 3" >&2
  FAIL=$(( FAIL + 1 ))
fi

rm -rf "${TMP_OPS_PR}" 2>/dev/null || true

# ---------------------------------------------------------------------------
# Test group: pre-tool-use.sh
# ---------------------------------------------------------------------------

echo ""
echo "=== pre-tool-use.sh behavior ==="

SAFE_PAYLOAD='{"tool_input":{"command":"git status"}}'
FORCE_PUSH_PAYLOAD='{"tool_input":{"command":"git push --force"}}'
RESET_HARD_PAYLOAD='{"tool_input":{"command":"git reset --hard"}}'
EMPTY_CMD_PAYLOAD='{"tool_input":{"command":""}}'

assert_exit 0 "pre-tool-use allows safe commands (git status)" \
  bash -c "echo '$SAFE_PAYLOAD' | bash '$HOOKS_DIR/pre-tool-use.sh'"

assert_exit 2 "pre-tool-use blocks git push --force" \
  bash -c "echo '$FORCE_PUSH_PAYLOAD' | bash '$HOOKS_DIR/pre-tool-use.sh'"

assert_exit 2 "pre-tool-use blocks git reset --hard" \
  bash -c "echo '$RESET_HARD_PAYLOAD' | bash '$HOOKS_DIR/pre-tool-use.sh'"

assert_exit 0 "pre-tool-use allows empty command field" \
  bash -c "echo '$EMPTY_CMD_PAYLOAD' | bash '$HOOKS_DIR/pre-tool-use.sh'"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0

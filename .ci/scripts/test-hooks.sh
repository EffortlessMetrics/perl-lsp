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

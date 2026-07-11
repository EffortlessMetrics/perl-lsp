#!/usr/bin/env bash
# Test the M4b read-only-agent shell guard in .claude/hooks/pre-tool-use.sh
# (#3763). When CLAUDE_AGENT_READONLY=1, mutating git/gh/filesystem commands
# must be rejected (exit 2) BEFORE execution, while read-only inspection
# commands pass (exit 0). Without the env flag, nothing extra is blocked.

set -eu

HOOK="$(git rev-parse --show-toplevel)/.claude/hooks/pre-tool-use.sh"
FAIL=0

# Build the tool-input JSON with jq so redirection / quote characters in the
# command are encoded safely.
run_case() {
  local label="$1" readonly_flag="$2" cmd="$3" expected_exit="$4"
  local actual
  actual=$(
    CLAUDE_AGENT_READONLY="$readonly_flag" \
      jq -nc --arg c "$cmd" '{tool_input:{command:$c}}' | \
      CLAUDE_AGENT_READONLY="$readonly_flag" bash "$HOOK" >/dev/null 2>&1
    echo $?
  )
  if [ "$actual" = "$expected_exit" ]; then
    echo "PASS  $label (exit $actual)"
  else
    echo "FAIL  $label (expected $expected_exit, got $actual) CMD=$cmd"
    FAIL=1
  fi
}

# --- read-only agent: mutating commands blocked ---
run_case "ro: git commit blocked"            1 'git commit -m wip'                    2
run_case "ro: git push blocked"              1 'git push origin HEAD'                 2
run_case "ro: git worktree add blocked"      1 'git worktree add ../x main'           2
run_case "ro: git add blocked"               1 'git add file.rs'                      2
run_case "ro: git checkout -b blocked"       1 'git checkout -b feature'              2
run_case "ro: gh pr comment blocked"         1 'gh pr comment 42 --body hi'           2
run_case "ro: gh pr review blocked"          1 'gh pr review 42 --approve'            2
run_case "ro: gh pr merge blocked"           1 'gh pr merge 42'                       2
run_case "ro: gh issue create blocked"       1 'gh issue create --title x'            2
run_case "ro: gh issue edit label blocked"   1 'gh issue edit 42 --add-label bug'     2
run_case "ro: gh api POST blocked"           1 'gh api -X POST repos/o/r/issues'      2
run_case "ro: file redirect blocked"         1 'echo hi > out.txt'                    2
run_case "ro: append redirect blocked"       1 'cat a >> b.txt'                       2
run_case "ro: tee blocked"                   1 'cargo test | tee log.txt'             2
run_case "ro: cp blocked"                    1 'cp a b'                               2
run_case "ro: sed -i blocked"                1 'sed -i s/a/b/ f'                       2

# --- read-only agent: inspection commands allowed ---
run_case "ro: git diff allowed"              1 'git diff origin/main'                 0
run_case "ro: git log allowed"               1 'git log --oneline -20'                0
run_case "ro: git status allowed"            1 'git status'                           0
run_case "ro: git show allowed"              1 'git show HEAD'                         0
run_case "ro: gh pr view allowed"            1 'gh pr view 42'                         0
run_case "ro: gh pr diff allowed"            1 'gh pr diff 42'                         0
run_case "ro: gh pr checks allowed"          1 'gh pr checks 42'                       0
run_case "ro: gh api GET allowed"            1 'gh api repos/o/r'                      0
run_case "ro: cargo check allowed"           1 'cargo check --workspace'              0
run_case "ro: stderr redirect allowed"       1 'cargo build 2>/dev/null'              0
run_case "ro: stderr dup allowed"            1 'cargo build 2>&1'                      0
run_case "ro: grep allowed"                  1 'grep -rn foo src'                      0

# --- flag off: writers are NOT restricted by this block ---
run_case "off: git commit allowed"           0 'git commit -m wip'                    0
run_case "off: gh pr comment allowed"        0 'gh pr comment 42 --body hi'           0
run_case "off: file redirect allowed"        0 'echo hi > out.txt'                    0

if [ "$FAIL" -eq 0 ]; then
  echo
  echo "All read-only-shell test cases passed."
  exit 0
else
  echo
  echo "Some read-only-shell test cases failed."
  exit 1
fi

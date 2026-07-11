#!/usr/bin/env bash
# Test the M4b publication-boundary guard in .claude/hooks/pre-tool-use.sh
# (#3763, "M4b publication-boundary moves DEFER -> BUILD"). The PreToolUse
# hook stdin payload carries `agent_type` when Claude Code invokes the hook
# inside a subagent (absent at the top level -> defaults to "main"). When
# `agent_type` names a review/audit persona (the same cohort
# `xtask/src/tasks/agent_capability_policy.rs` forbids from Edit/Write), the
# hook must deny the genuinely-irreversible publish actions -- `git push` in
# any form and `gh pr merge` -- BEFORE execution, regardless of
# CLAUDE_AGENT_READONLY. Writers (agent_type=builder, etc.) and the
# top-level orchestrator (agent_type absent or "main") must be unaffected --
# the orchestrator still needs `gh pr merge --auto` to work.

set -eu

HOOK="$(git rev-parse --show-toplevel)/.claude/hooks/pre-tool-use.sh"
FAIL=0

# Build the PreToolUse JSON payload with jq so command/agent_type quoting is
# safe. `agent_type` is only included in the payload when the case supplies
# one (mirrors the real hook: absent field -> `main` at the top level).
run_case() {
  local label="$1" agent_type="$2" cmd="$3" expected_exit="$4"
  local actual payload

  if [ -n "$agent_type" ]; then
    payload=$(jq -nc --arg t "$agent_type" --arg c "$cmd" '{agent_type:$t, tool_input:{command:$c}}')
  else
    payload=$(jq -nc --arg c "$cmd" '{tool_input:{command:$c}}')
  fi

  actual=$(printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>&1; echo $?)

  if [ "$actual" = "$expected_exit" ]; then
    echo "PASS  $label (exit $actual)"
  else
    echo "FAIL  $label (expected $expected_exit, got $actual) agent_type=$agent_type CMD=$cmd"
    FAIL=1
  fi
}

# Same as run_case, but builds the payload with `subagent_type` instead of
# `agent_type` -- proves the defensive fallback (`.agent_type // .subagent_type
# // "main"`) actually engages when only `subagent_type` is present.
run_case_subagent_type_only() {
  local label="$1" subagent_type="$2" cmd="$3" expected_exit="$4"
  local actual payload

  payload=$(jq -nc --arg t "$subagent_type" --arg c "$cmd" '{subagent_type:$t, tool_input:{command:$c}}')
  actual=$(printf '%s' "$payload" | bash "$HOOK" >/dev/null 2>&1; echo $?)

  if [ "$actual" = "$expected_exit" ]; then
    echo "PASS  $label (exit $actual)"
  else
    echo "FAIL  $label (expected $expected_exit, got $actual) subagent_type=$subagent_type CMD=$cmd"
    FAIL=1
  fi
}

# --- (a) review/audit persona: git push blocked ---
run_case "reviewer-deep: git push blocked"       "reviewer-deep" "git push origin HEAD"     2

# --- (b) review/audit persona: gh pr merge blocked ---
run_case "diff-auditor: gh pr merge blocked"      "diff-auditor" "gh pr merge --auto 3763"   2

# --- (c) writer persona: git push allowed ---
run_case "builder: git push allowed"              "builder" "git push"                       0

# --- (d) top-level orchestrator (agent_type=main): gh pr merge allowed ---
run_case "main: gh pr merge --auto allowed"        "main" "gh pr merge --auto"                0

# --- (e) top-level orchestrator (agent_type absent): gh pr merge allowed ---
run_case "absent agent_type: gh pr merge --auto allowed" "" "gh pr merge --auto"               0

# --- additional coverage: every review/audit agent_type is covered, both
#     publish verbs, and a bare 'git push' with no args is caught too ---
run_case "plan-reviewer: bare git push blocked"    "plan-reviewer" "git push"                 2
run_case "architecture-reviewer: gh pr merge blocked" "architecture-reviewer" "gh pr merge 42" 2
run_case "reviewer: git push --force still blocked" "reviewer" "git push --force origin main" 2

# --- non-publish actions from a review/audit persona remain allowed by this
#     guard (other guards in the hook may still apply independently) ---
run_case "reviewer-deep: git diff allowed"         "reviewer-deep" "git diff origin/main"     0
run_case "reviewer-deep: gh pr view allowed"       "reviewer-deep" "gh pr view 42"             0
run_case "reviewer-deep: gh pr comment allowed"    "reviewer-deep" "gh pr comment 42 --body hi" 0

# --- global-option bypass forms (2026-07-11 deep-review finding on #3808):
#     a leading git/gh global option must not hide the subcommand from the
#     guard ---
run_case "reviewer-deep: git -C <dir> push blocked"   "reviewer-deep" 'git -C /x push'          2
run_case "reviewer-deep: git --no-pager push blocked" "reviewer-deep" 'git --no-pager push'     2
run_case "diff-auditor: gh --repo pr merge blocked"   "diff-auditor"  'gh --repo o/n pr merge 5' 2

# --- shell-separator / subshell-wrap bypass forms (same deep-review finding):
#     a bare `([[:space:]]|$)` terminator let a trailing separator or a
#     closing paren ride straight through "push" uncaught ---
run_case "reviewer-deep: git push;echo blocked"    "reviewer-deep" 'git push;echo hi'          2
run_case "reviewer-deep: git push&&x blocked"      "reviewer-deep" 'git push&&x'                2
run_case "reviewer-deep: (git push) blocked"       "reviewer-deep" '(git push)'                 2

# --- KNOWN LIMITATION, documented not hidden (see the hook's "Known,
#     accepted limitations" comment): this guard is a regex over the literal
#     command string. Shell indirection -- `sh -c "..."`, `eval`, a
#     decode-and-pipe, or a script written to disk and executed separately --
#     is NOT caught. This is an accepted gap, not a claim of adversarial
#     sandboxing; expected result is ALLOW (exit 0), not a bug.
run_case "KNOWN LIMITATION: sh -c 'git push' not caught" "reviewer-deep" 'sh -c "git push"'     0

# --- subagent_type fallback (2026-07-11): agent_type is the confirmed,
#     PRIMARY PreToolUse field (captured live from a real diff-auditor
#     persona: agent_type="diff-auditor" populated, subagent_type null).
#     subagent_type is read as a defensive fallback for other event shapes /
#     future harness versions. A payload carrying ONLY subagent_type (no
#     agent_type at all) must still be recognized and denied. ---
run_case_subagent_type_only "subagent_type-only fallback: reviewer-deep git push blocked" "reviewer-deep" "git push" 2

if [ "$FAIL" -eq 0 ]; then
  echo
  echo "All publish-boundary test cases passed."
  exit 0
else
  echo
  echo "Some publish-boundary test cases failed."
  exit 1
fi

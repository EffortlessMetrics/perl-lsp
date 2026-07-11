#!/usr/bin/env bash
# Drift guard for the M4b publication-boundary (#3763): the review/audit
# agent_type list hardcoded in .claude/hooks/pre-tool-use.sh (the set that
# may never `git push` / `gh pr merge`) MUST stay in sync with
# xtask/src/tasks/agent_capability_policy.rs's REVIEW_AUDIT_AGENTS -- the
# same list that already governs the Edit/Write tool-allowlist boundary
# (#3771). Two independently-maintained copies of this list is exactly the
# drift risk the spec calls out; this test fails loudly the moment they
# disagree so nobody has to notice by hand.

set -eu

ROOT="$(git rev-parse --show-toplevel)"
HOOK="$ROOT/.claude/hooks/pre-tool-use.sh"
POLICY="$ROOT/xtask/src/tasks/agent_capability_policy.rs"

# Extract the hook's REVIEW_AUDIT_AGENT_TYPES="a b c" assignment.
hook_list=$(grep -oE '^REVIEW_AUDIT_AGENT_TYPES="[^"]*"' "$HOOK" \
  | sed -E 's/^REVIEW_AUDIT_AGENT_TYPES="//; s/"$//')

if [ -z "$hook_list" ]; then
  echo "FAIL: could not find REVIEW_AUDIT_AGENT_TYPES=\"...\" in $HOOK" >&2
  exit 1
fi

# Extract the Rust `pub const REVIEW_AUDIT_AGENTS: &[&str] = &[ ... ];` body
# and pull out the quoted string literals.
policy_list=$(awk '/pub const REVIEW_AUDIT_AGENTS: &\[&str\] = &\[/{flag=1; next} /\];/{flag=0} flag' "$POLICY" \
  | grep -oE '"[a-zA-Z0-9_-]+"' \
  | tr -d '"')

if [ -z "$policy_list" ]; then
  echo "FAIL: could not find REVIEW_AUDIT_AGENTS list in $POLICY" >&2
  exit 1
fi

hook_sorted=$(printf '%s\n' "$hook_list" | tr ' ' '\n' | sort -u)
policy_sorted=$(printf '%s\n' "$policy_list" | sort -u)

if [ "$hook_sorted" = "$policy_sorted" ]; then
  count=$(printf '%s\n' "$hook_sorted" | wc -l | tr -d ' ')
  echo "PASS  hook REVIEW_AUDIT_AGENT_TYPES matches xtask REVIEW_AUDIT_AGENTS ($count agents)"
  exit 0
else
  echo "FAIL: .claude/hooks/pre-tool-use.sh's REVIEW_AUDIT_AGENT_TYPES has drifted from" >&2
  echo "      xtask/src/tasks/agent_capability_policy.rs's REVIEW_AUDIT_AGENTS." >&2
  echo "--- hook only ---" >&2
  comm -23 <(printf '%s\n' "$hook_sorted") <(printf '%s\n' "$policy_sorted") >&2
  echo "--- policy only ---" >&2
  comm -13 <(printf '%s\n' "$hook_sorted") <(printf '%s\n' "$policy_sorted") >&2
  exit 1
fi

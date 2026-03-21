#!/usr/bin/env bash
# PreToolUse hook: block dangerous bash commands before execution
# Exit 2 = block with feedback
# Exit 0 = allow

INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if echo "$CMD" | grep -qE 'git push --force|git push -f |git checkout \.|git reset --hard|rm -rf /|cargo publish|git clean -fd'; then
  echo "Blocked: dangerous command '$CMD'. Use safer alternatives." >&2
  exit 2
fi

exit 0

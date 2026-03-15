#!/usr/bin/env bash
#
# Bootstrap script for the agent swarm workflow.
#
# Copies portable slash commands into .claude/commands/ and creates a
# minimal .claude/settings.json with a PostToolUse hook.
#
# Usage:
#   bash docs/handoff/agent-swarm-workflow/setup-script.sh
#
# Run from your repository root.

set -euo pipefail

# --- Configuration ---------------------------------------------------------
# Override these if your project uses different commands.

# The command that runs after every Edit/Write to catch errors early.
# Examples:
#   Rust:       "cargo check --quiet --message-format=short 2>&1 | head -20 || true"
#   Python:     "python -m py_compile \"$FILE\" 2>&1 | head -20 || true"
#   TypeScript: "npx tsc --noEmit 2>&1 | head -20 || true"
#   Go:         "go vet ./... 2>&1 | head -20 || true"
POST_EDIT_CHECK="${POST_EDIT_CHECK:-cargo check --quiet --message-format=short 2>&1 | head -20 || true}"

# --- Resolve paths ---------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SLASH_CMD_SRC="${SCRIPT_DIR}/slash-commands"
REPO_ROOT="$(pwd)"
CLAUDE_DIR="${REPO_ROOT}/.claude"
CMD_DIR="${CLAUDE_DIR}/commands"
SETTINGS="${CLAUDE_DIR}/settings.json"

# --- Pre-flight checks -----------------------------------------------------

if [ ! -d "${SLASH_CMD_SRC}" ]; then
    echo "ERROR: Cannot find slash-commands/ directory at ${SLASH_CMD_SRC}"
    echo "       Run this script from the repository root, or set SCRIPT_DIR."
    exit 1
fi

# --- Create directories ----------------------------------------------------

echo "Creating .claude/commands/ ..."
mkdir -p "${CMD_DIR}"

# --- Copy slash commands ----------------------------------------------------

echo "Copying slash command templates ..."

for src_file in "${SLASH_CMD_SRC}"/*.md; do
    filename="$(basename "$src_file")"
    dest="${CMD_DIR}/${filename}"

    if [ -f "$dest" ]; then
        echo "  SKIP: ${filename} (already exists, not overwriting)"
    else
        cp "$src_file" "$dest"
        echo "  COPY: ${filename}"
    fi
done

# --- Create settings.json --------------------------------------------------

if [ -f "$SETTINGS" ]; then
    echo ""
    echo "SKIP: .claude/settings.json already exists."
    echo "      Review it manually and add PostToolUse hooks if needed."
    echo "      Recommended hook command: ${POST_EDIT_CHECK}"
else
    echo ""
    echo "Creating .claude/settings.json ..."
    cat > "$SETTINGS" <<SETTINGSEOF
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|NotebookEdit",
        "hooks": [
          {
            "type": "command",
            "command": "${POST_EDIT_CHECK}"
          }
        ]
      }
    ]
  }
}
SETTINGSEOF
    echo "  Created with PostToolUse hook: ${POST_EDIT_CHECK}"
fi

# --- Print instructions -----------------------------------------------------

echo ""
echo "========================================================================"
echo " Agent Swarm Workflow -- Setup Complete"
echo "========================================================================"
echo ""
echo " Files created in: ${CMD_DIR}/"
echo ""
echo " Next steps:"
echo ""
echo "   1. Edit the slash commands in .claude/commands/ to replace"
echo "      placeholder variables with your project's commands:"
echo ""
echo "        \$TEST_CMD   -- your test runner       (e.g., cargo test, pytest)"
echo "        \$LINT_CMD   -- your linter             (e.g., cargo clippy, ruff)"
echo "        \$FMT_CMD    -- your formatter           (e.g., cargo fmt, prettier)"
echo "        \$BUILD_CMD  -- your build command       (e.g., cargo build, npm build)"
echo "        \$CHECK_CMD  -- fast type/compile check  (e.g., cargo check, tsc)"
echo "        \$GATE_CMD   -- full CI gate command     (e.g., just ci-gate, make ci)"
echo ""
echo "   2. Review .claude/settings.json and adjust the PostToolUse hook"
echo "      command if needed."
echo ""
echo "   3. Start Claude Code and try:"
echo "        /wave test-coverage     -- launch a test coverage wave"
echo "        /tdd-fix <bug>          -- fix a bug with TDD"
echo "        /bulk-pr                -- PR all worktrees at once"
echo ""
echo "   4. (Optional) Add .claude/ to .gitignore if you do not want"
echo "      to check in agent configuration, or commit it to share"
echo "      with your team."
echo ""
echo "   5. Read docs/handoff/agent-swarm-workflow/agent-patterns.md"
echo "      for tips on effective agent dispatch."
echo ""
echo "========================================================================"

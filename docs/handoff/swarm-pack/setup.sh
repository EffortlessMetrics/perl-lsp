#!/usr/bin/env bash
#
# Swarm Pack Setup Script
#
# Installs the swarm agent infrastructure into your repository.
# Copies agents, commands, hooks, and creates a queue artifact.
#
# Usage:
#   bash path/to/swarm-pack/setup.sh
#
# Run from your repository root.
#
# Environment variables (optional overrides):
#   FMT_CMD          - format command         (default: "cargo fmt --all")
#   FMT_CHECK_CMD    - format check command   (default: "cargo fmt --all -- --check")
#   LINT_CMD         - lint command            (default: "cargo clippy -p PKG --tests -- -D warnings")
#   TEST_CMD         - test command            (default: "cargo test -p PKG")
#   POST_EDIT_CHECK  - PostToolUse hook cmd    (default: "cargo check --quiet ...")
#   STATUS_REGEN_CMD - status regen command    (default: "echo 'TODO: set STATUS_REGEN_CMD'")
#   BASELINE_RATCHET_CMD - baseline ratchet    (default: "echo 'TODO: set BASELINE_RATCHET_CMD'")
#   OPS_DIR          - ops directory name      (default: ".ops")
#   MAIN_BRANCH      - main branch name        (default: "main")

set -euo pipefail

# --- Configuration -----------------------------------------------------------

FMT_CMD="${FMT_CMD:-cargo fmt --all}"
FMT_CHECK_CMD="${FMT_CHECK_CMD:-cargo fmt --all -- --check}"
LINT_CMD="${LINT_CMD:-cargo clippy -p PKG --tests -- -D warnings}"
TEST_CMD="${TEST_CMD:-cargo test -p PKG}"
POST_EDIT_CHECK="${POST_EDIT_CHECK:-cargo check --quiet --message-format=short 2>&1 | head -20 || true}"
STATUS_REGEN_CMD="${STATUS_REGEN_CMD:-echo 'TODO: set STATUS_REGEN_CMD'}"
BASELINE_RATCHET_CMD="${BASELINE_RATCHET_CMD:-echo 'TODO: set BASELINE_RATCHET_CMD'}"
OPS_DIR="${OPS_DIR:-.ops}"
MAIN_BRANCH="${MAIN_BRANCH:-main}"

# --- Resolve paths -----------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(pwd)"
CLAUDE_DIR="${REPO_ROOT}/.claude"

# --- Pre-flight checks -------------------------------------------------------

if [ ! -d "${SCRIPT_DIR}/agents" ]; then
    echo "ERROR: Cannot find agents/ directory at ${SCRIPT_DIR}/agents"
    echo "       Run this script from your repository root."
    exit 1
fi

# --- Create directories ------------------------------------------------------

echo "Creating directories..."
mkdir -p "${CLAUDE_DIR}/agents"
mkdir -p "${CLAUDE_DIR}/commands"
mkdir -p "${CLAUDE_DIR}/hooks"
mkdir -p "${REPO_ROOT}/${OPS_DIR}"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/handoffs"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/salvage"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/agent-patches"

# --- Install protocol as a skill -----------------------------------------------

PROTOCOL_SRC="${SCRIPT_DIR}/SWARM_PROTOCOL.md"
PROTOCOL_DEST="${CLAUDE_DIR}/commands/swarm-protocol.md"
if [ -f "$PROTOCOL_SRC" ] && [ ! -f "$PROTOCOL_DEST" ]; then
    {
        printf '%s\n' '---' 'description: Load swarm behavioral rules' 'argument-hint: ""' '---' ''
        cat "$PROTOCOL_SRC"
    } > "$PROTOCOL_DEST"
    echo "COPY: swarm-protocol.md (as /swarm-protocol skill)"
elif [ -f "$PROTOCOL_DEST" ]; then
    echo "SKIP: swarm-protocol.md (exists)"
fi

# --- Copy agents (ALL .md files, not just swarm-*) ----------------------------

echo ""
echo "Installing agents..."
for src_file in "${SCRIPT_DIR}"/agents/*.md; do
    filename="$(basename "$src_file")"
    dest="${CLAUDE_DIR}/agents/${filename}"
    if [ -f "$dest" ]; then
        echo "  SKIP: ${filename} (exists)"
    else
        cp "$src_file" "$dest"
        echo "  COPY: ${filename}"
    fi
done

# --- Copy commands ------------------------------------------------------------

echo ""
echo "Installing commands..."
for src_file in "${SCRIPT_DIR}"/commands/*.md; do
    filename="$(basename "$src_file")"
    dest="${CLAUDE_DIR}/commands/${filename}"
    if [ -f "$dest" ]; then
        echo "  SKIP: ${filename} (exists)"
    else
        cp "$src_file" "$dest"
        echo "  COPY: ${filename}"
    fi
done

# --- Copy hooks ---------------------------------------------------------------

echo ""
echo "Installing hooks..."
for src_file in "${SCRIPT_DIR}"/hooks/*.sh; do
    filename="$(basename "$src_file")"
    dest="${CLAUDE_DIR}/hooks/${filename}"
    if [ -f "$dest" ]; then
        echo "  SKIP: ${filename} (exists)"
    else
        cp "$src_file" "$dest"
        chmod +x "$dest"
        echo "  COPY: ${filename}"
    fi
done

# --- Create tracked knowledge files (.claude/swarm-state/) --------------------
# These are tracked in git — they persist across sessions and developers.

SWARM_STATE="${CLAUDE_DIR}/swarm-state"
mkdir -p "${SWARM_STATE}"

for artifact in swarm-queue.json known-pitfalls.md completed-slices.md discovered-issues.md; do
    dest="${SWARM_STATE}/${artifact}"
    if [ -f "$dest" ]; then
        echo "  SKIP: swarm-state/${artifact} (exists)"
    else
        case "$artifact" in
            swarm-queue.json)
                cat > "$dest" <<'ARTEOF'
{"_comment":"Overlap tracking for swarm coordinators","slices":[],"hot_files":[]}
ARTEOF
                ;;
            known-pitfalls.md)
                cat > "$dest" <<'ARTEOF'
# Known Pitfalls
Accumulated lessons from fixer agents. Scouts and builders read this to avoid repeating known mistakes.
<!-- Agents append below -->
ARTEOF
                ;;
            completed-slices.md)
                cat > "$dest" <<'ARTEOF'
# Completed Slices
Scouts check this before creating tasks to avoid rediscovering finished work.
Format: `- <branch> | <category> | <packages> | <status> | <description>`
<!-- Agents append below -->
ARTEOF
                ;;
            discovered-issues.md)
                cat > "$dest" <<'ARTEOF'
# Discovered Issues
Any agent can append here when they notice something outside their scope.
<!-- Agents append below -->
ARTEOF
                ;;
        esac
        echo "  CREATED: swarm-state/${artifact}"
    fi
done

# --- Create ephemeral runtime dirs (.ops/) ------------------------------------
# These are gitignored — per-session runtime data only.

echo ""
echo "Creating ephemeral runtime directories (gitignored)..."

# --- Create GitHub labels (if gh is available) --------------------------------

if command -v gh &>/dev/null && gh auth status &>/dev/null 2>&1; then
    echo ""
    echo "Creating GitHub labels..."
    for label in "swarm-core:0E8A16:Primary swarm task" \
                 "swarm-improve-docs:C5DEF5:Documentation improvement" \
                 "swarm-improve-tests:C5DEF5:Test quality improvement" \
                 "swarm-improve-devex:C5DEF5:Developer experience improvement" \
                 "swarm-improve-infra:C5DEF5:Infrastructure improvement" \
                 "swarm-discovered:FBCA04:Issue found by swarm agent" \
                 "swarm-architectural:D93F0B:Needs architectural decision"; do
        IFS=: read -r name color desc <<< "$label"
        if gh label create "$name" --color "$color" --description "$desc" 2>/dev/null; then
            echo "  CREATED: $name"
        else
            echo "  EXISTS:  $name"
        fi
    done
else
    echo ""
    echo "SKIP: GitHub labels (gh CLI not available or not authenticated)"
    echo "  Create these labels manually: swarm-core, swarm-side-fix,"
    echo "  swarm-improve-docs, swarm-improve-tests, swarm-improve-devex,"
    echo "  swarm-improve-infra, swarm-discovered, swarm-architectural"
fi

# --- Create or merge settings.json -------------------------------------------

SETTINGS="${CLAUDE_DIR}/settings.json"
echo ""
if [ -f "$SETTINGS" ]; then
    echo "EXISTING: .claude/settings.json found."
    echo "  Add these hooks manually if not already present:"
    echo ""
    echo '  "TeammateIdle": [{"hooks": [{"type": "command", "command": "bash .claude/hooks/teammate-idle.sh"}]}],'
    echo '  "TaskCompleted": [{"hooks": [{"type": "command", "command": "bash .claude/hooks/task-completed.sh"}]}]'
    echo ""
else
    echo "Creating .claude/settings.json..."
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
    ],
    "TeammateIdle": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/teammate-idle.sh"
          }
        ]
      }
    ],
    "TaskCompleted": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash .claude/hooks/task-completed.sh"
          }
        ]
      }
    ]
  }
}
SETTINGSEOF
    echo "  Created with PostToolUse, TeammateIdle, and TaskCompleted hooks"
fi

# --- Print customization guide -----------------------------------------------

echo ""
echo "========================================================================"
echo " Swarm Pack — Setup Complete"
echo "========================================================================"
echo ""
AGENT_COUNT=$(ls -1 "${CLAUDE_DIR}/agents"/*.md 2>/dev/null | wc -l)
SKILL_COUNT=$(ls -1 "${CLAUDE_DIR}/commands"/*.md 2>/dev/null | wc -l)
echo " Installed:"
echo "   - ${AGENT_COUNT} agent definitions in .claude/agents/"
echo "   - ${SKILL_COUNT} skills in .claude/commands/"
echo "   - 2 hooks in .claude/hooks/"
echo "   - .claude/swarm-state/  — tracked knowledge (pitfalls, slices, discoveries, queue)"
echo "   - ${OPS_DIR}/           — ephemeral runtime (handoffs, metrics, patches, salvage)"
echo "   - GitHub labels (7)"
echo ""
echo " Next steps — customize for your project:"
echo ""
echo "   1. AGENT DEFINITIONS (.claude/agents/swarm-*.md):"
echo "      Replace placeholder variables with your commands:"
echo ""
echo "        \$FMT_CMD          → ${FMT_CMD}"
echo "        \$FMT_CHECK_CMD    → ${FMT_CHECK_CMD}"
echo "        \$LINT_CMD          → ${LINT_CMD}"
echo "        \$TEST_CMD          → ${TEST_CMD}"
echo "        \$DEAD_CODE_CMD     → your dead code detector"
echo "        \$UNUSED_DEPS_CMD   → your unused deps checker"
echo ""
echo "   2. SCOUT FOCUS AREAS (.claude/agents/swarm-scout.md):"
echo "      Replace \$ERROR_SOURCE, \$TEST_GAPS, etc. with your:"
echo "        - Bug tracking / error baseline sources"
echo "        - Test coverage gap locations"
echo "        - Technical debt tracking file"
echo ""
echo "   3. DRIFT COMMANDS (.claude/agents/swarm-merger.md, commands/status-drift.md):"
echo "      Replace \$STATUS_REGEN_CMD and \$BASELINE_RATCHET_CMD"
echo ""
echo "   4. FORMAT CHECK HOOK (.claude/hooks/task-completed.sh):"
echo "      Replace FMT_CHECK_CMD default with your formatter"
echo ""
echo "   5. ENABLE AGENT TEAMS in ~/.claude/settings.json:"
echo '      { "env": { "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1" } }'
echo ""
echo "   6. GENERATE DOMAIN AGENTS (recommended):"
echo "      claude"
echo "      /bootstrap-agents"
echo ""
echo "      This discovers your codebase and generates ~25-30 domain-specific"
echo "      agent definitions with your actual package paths, test commands,"
echo "      error sources, and coding standards pre-encoded."
echo ""
echo "   7. START THE SWARM:"
echo "      /swarm all"
echo ""
echo "   8. (Optional) Customize main branch name in commands if not 'main'"
echo "      Current commands use: origin/${MAIN_BRANCH}"
echo ""
echo "========================================================================"

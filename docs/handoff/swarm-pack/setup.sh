#!/usr/bin/env bash
#
# Swarm Pack Setup Script
#
# Installs the swarm agent infrastructure into your repository.
# Copies agents, skills, commands, hooks, and creates a queue artifact.
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
#   STATUS_REGEN_CMD - status regen command    (default: "echo 'set STATUS_REGEN_CMD'")
#   BASELINE_RATCHET_CMD - baseline ratchet    (default: "echo 'set BASELINE_RATCHET_CMD'")
#   OPS_DIR          - ops directory name      (default: ".ops")
#   MAIN_BRANCH      - main branch name        (default: "main")

set -euo pipefail
shopt -s nullglob   # globs that match nothing expand to empty (not literal pattern)

# --- Configuration -----------------------------------------------------------

FMT_CMD="${FMT_CMD:-cargo fmt --all}"
FMT_CHECK_CMD="${FMT_CHECK_CMD:-cargo fmt --all -- --check}"
LINT_CMD="${LINT_CMD:-cargo clippy -p PKG --tests -- -D warnings}"
TEST_CMD="${TEST_CMD:-cargo test -p PKG}"
POST_EDIT_CHECK="${POST_EDIT_CHECK:-cargo check --quiet --message-format=short 2>&1 | head -20 || true}"
STATUS_REGEN_CMD="${STATUS_REGEN_CMD:-echo 'set STATUS_REGEN_CMD'}"
BASELINE_RATCHET_CMD="${BASELINE_RATCHET_CMD:-echo 'set BASELINE_RATCHET_CMD'}"
OPS_DIR="${OPS_DIR:-.ops}"
MAIN_BRANCH="${MAIN_BRANCH:-main}"

# --- Resolve paths -----------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(pwd)"
CLAUDE_DIR="${REPO_ROOT}/.claude"

# --- Pre-flight checks -------------------------------------------------------

if [ ! -d "${SCRIPT_DIR}/agents" ] && [ ! -d "${SCRIPT_DIR}/commands" ] && [ ! -d "${SCRIPT_DIR}/skills" ]; then
    echo "ERROR: Cannot find agents/, commands/, or skills/ in pack directory: ${SCRIPT_DIR}"
    echo "       Make sure you're pointing at the swarm-pack/ directory."
    exit 1
fi

# --- Create directories ------------------------------------------------------

echo "Creating directories..."
mkdir -p "${CLAUDE_DIR}/agents"
mkdir -p "${CLAUDE_DIR}/skills"
mkdir -p "${CLAUDE_DIR}/commands"
mkdir -p "${CLAUDE_DIR}/hooks"
mkdir -p "${REPO_ROOT}/${OPS_DIR}"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/handoffs"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/salvage"
mkdir -p "${REPO_ROOT}/${OPS_DIR}/agent-patches"

# --- Install protocol as a command ---------------------------------------------

PROTOCOL_SRC="${SCRIPT_DIR}/SWARM_PROTOCOL.md"
PROTOCOL_DEST="${CLAUDE_DIR}/commands/swarm-protocol.md"
if [ -f "$PROTOCOL_DEST" ]; then
    echo "SKIP: swarm-protocol.md (exists)"
elif [ -f "$PROTOCOL_SRC" ]; then
    {
        printf '%s\n' '---' 'description: Load swarm behavioral rules' 'argument-hint: ""' '---' ''
        cat "$PROTOCOL_SRC"
    } > "$PROTOCOL_DEST"
    echo "COPY: swarm-protocol.md (as /swarm-protocol command)"
else
    echo "SKIP: SWARM_PROTOCOL.md not found in pack — install /swarm-protocol manually"
fi

# --- Copy skills --------------------------------------------------------------

echo ""
echo "Installing skills..."
if [ ! -d "${SCRIPT_DIR}/skills" ]; then
    echo "  SKIP: no skills/ directory in pack"
else
    for src_dir in "${SCRIPT_DIR}"/skills/*; do
        [ -d "${src_dir}" ] || continue
        skill_name="$(basename "${src_dir}")"
        dest="${CLAUDE_DIR}/skills/${skill_name}"
        if [ -e "${dest}" ]; then
            echo "  SKIP: ${skill_name}/ (exists)"
        else
            cp -R "${src_dir}" "${dest}"
            echo "  COPY: ${skill_name}/"
        fi
    done
fi

# --- Copy agents (ALL .md files, not just swarm-*) ----------------------------

echo ""
echo "Installing agents..."
if [ ! -d "${SCRIPT_DIR}/agents" ]; then
    echo "  SKIP: no agents/ directory in pack"
else
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
fi

# --- Copy commands ------------------------------------------------------------

echo ""
echo "Installing commands..."
if [ ! -d "${SCRIPT_DIR}/commands" ]; then
    echo "  SKIP: no commands/ directory in pack"
else
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
fi

# --- Copy hooks ---------------------------------------------------------------

echo ""
echo "Installing hooks..."
if [ ! -d "${SCRIPT_DIR}/hooks" ]; then
    echo "  SKIP: no hooks/ directory in pack"
else
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
fi

# --- Create tracked knowledge files (.claude/swarm-state/) --------------------
# These are tracked in git — they persist across sessions and developers.

SWARM_STATE="${CLAUDE_DIR}/swarm-state"
mkdir -p "${SWARM_STATE}"

TODAY="$(date +%F)"

for artifact in swarm-queue.json known-pitfalls.md completed-slices.md discovered-issues.md findings.json findings.schema.json; do
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
            findings.json)
                cat > "$dest" <<ARTEOF
{
  "_comment": "Durable control-plane findings for the tracked swarm surface. Product bugs belong in discovered-issues.md or GitHub issues instead.",
  "schema_version": 1,
  "last_updated": "${TODAY}",
  "findings": []
}
ARTEOF
                ;;
            findings.schema.json)
                cat > "$dest" <<'ARTEOF'
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Swarm Findings Ledger",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "schema_version",
    "last_updated",
    "findings"
  ],
  "properties": {
    "_comment": {
      "type": "string"
    },
    "schema_version": {
      "const": 1
    },
    "last_updated": {
      "type": "string",
      "format": "date"
    },
    "findings": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/finding"
      }
    }
  },
  "$defs": {
    "finding": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "id",
        "kind",
        "status",
        "recorded_on",
        "summary",
        "decision",
        "surfaces",
        "evidence",
        "follow_up"
      ],
      "properties": {
        "id": {
          "type": "string",
          "pattern": "^SWARM-FINDING-[0-9]{4}$"
        },
        "kind": {
          "type": "string",
          "enum": [
            "control_plane",
            "runtime_invariant",
            "docs_drift",
            "workflow_gap",
            "tracking_gap"
          ]
        },
        "status": {
          "type": "string",
          "enum": [
            "active",
            "landed",
            "watching",
            "superseded"
          ]
        },
        "recorded_on": {
          "type": "string",
          "format": "date"
        },
        "summary": {
          "type": "string",
          "minLength": 1
        },
        "decision": {
          "type": "string",
          "minLength": 1
        },
        "surfaces": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "string",
            "minLength": 1
          }
        },
        "evidence": {
          "type": "array",
          "minItems": 1,
          "items": {
            "$ref": "#/$defs/evidence"
          }
        },
        "follow_up": {
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1
          }
        },
        "notes": {
          "type": "string"
        }
      }
    },
    "evidence": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "type",
        "ref"
      ],
      "properties": {
        "type": {
          "type": "string",
          "enum": [
            "file",
            "pr",
            "issue",
            "doc",
            "hook",
            "setting"
          ]
        },
        "ref": {
          "type": "string",
          "minLength": 1
        }
      }
    }
  }
}
ARTEOF
                ;;
        esac
        echo "  CREATED: swarm-state/${artifact}"
    fi
done

# --- Gitignore ephemeral runtime dirs (.ops/) ---------------------------------
# The ops dirs were created above; now make sure they're gitignored.

GITIGNORE="${REPO_ROOT}/.gitignore"
GITIGNORE_ENTRY="${OPS_DIR}/"
echo ""
if [ -f "$GITIGNORE" ] && grep -qF "$GITIGNORE_ENTRY" "$GITIGNORE"; then
    echo "SKIP: .gitignore entry for ${GITIGNORE_ENTRY} (already present)"
else
    echo "${GITIGNORE_ENTRY}" >> "$GITIGNORE"
    echo "ADDED: .gitignore entry for ${GITIGNORE_ENTRY}"
fi

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
    echo "  Create these labels manually:"
    echo "  swarm-core, swarm-improve-docs, swarm-improve-tests,"
    echo "  swarm-improve-devex, swarm-improve-infra, swarm-discovered, swarm-architectural"
fi

# --- Create or merge settings.json -------------------------------------------

SETTINGS="${CLAUDE_DIR}/settings.json"
echo ""
if [ -f "$SETTINGS" ]; then
    echo "EXISTING: .claude/settings.json found."
    echo "  Add these hooks manually if not already present:"
    echo ""
    echo '  "TeammateIdle": [{"hooks": [{"type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/teammate-idle.sh"}]}],'
    echo '  "TaskCompleted": [{"hooks": [{"type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/task-completed.sh"}]}],'
    echo '  "SubagentStart": [{"matcher": "builder|reviewer|fixer|validator|bootstrapper|pr-responder|ops|improver", "hooks": [{"type": "command", "command": "echo '\''Reminder: Invoke /coding-standards before writing code.'\''"}]}],'
    echo '  "SubagentStop": [{"matcher": "builder|reviewer|fixer|validator|bootstrapper|pr-responder|ops|improver", "hooks": [{"type": "command", "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/subagent-stop.sh"}]}],'
    echo '  "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "INPUT=$(cat); CMD=$(echo \"$INPUT\" | jq -r '\''.tool_input.command // empty'\''); ..."}]}]'
    echo '  "SessionStart": [{"matcher": "compact", "hooks": [{"type": "command", "command": "echo '\''Post-compaction context refresh...'\''"}]}]'
    echo '  WorktreeCreate/WorktreeRemove are intentionally omitted from the shared template because they replace Claude Code'\''s default git worktree behavior.'
    echo ""
    echo "  See the generated settings.json template in this setup.sh for full hook commands."
    echo ""
else
    echo "Creating .claude/settings.json..."
    cat > "$SETTINGS" <<SETTINGSEOF
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  },
  "permissions": {
    "allow": [
      "Bash(gh *)",
      "Bash(git *)",
      "Bash(bash *)",
      "Bash(python3 *)",
      "Bash(mkdir *)",
      "Bash(cp *)",
      "Bash(ls *)",
      "Bash(grep *)",
      "Bash(echo *)",
      "Bash(chmod *)",
      "Bash(find *)",
      "WebFetch",
      "WebSearch"
    ]
  },
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
            "command": "\"\$CLAUDE_PROJECT_DIR\"/.claude/hooks/teammate-idle.sh"
          }
        ]
      }
    ],
    "TaskCompleted": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"\$CLAUDE_PROJECT_DIR\"/.claude/hooks/task-completed.sh"
          }
        ]
      }
    ],
    "SubagentStart": [
      {
        "matcher": "builder|reviewer|fixer|validator|bootstrapper|pr-responder|ops|improver",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Reminder: Invoke /coding-standards before writing any code. No unwrap(), expect(), panic!() in production code. Run fmt + lint before committing.'"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "builder|reviewer|fixer|validator|bootstrapper|pr-responder|ops|improver",
        "hooks": [
          {
            "type": "command",
            "command": "\"\$CLAUDE_PROJECT_DIR\"/.claude/hooks/subagent-stop.sh"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "INPUT=\$(cat); CMD=\$(echo \"\$INPUT\" | jq -r '.tool_input.command // empty'); if echo \"\$CMD\" | grep -qE 'git push --force|git push -f |git checkout \\\\.|git reset --hard|rm -rf /'; then echo \"Blocked: dangerous command '\$CMD'.\" >&2; exit 2; fi; exit 0"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "compact",
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Post-compaction context refresh: swarm mode is worktree-first and worker-disposable. Coordinators route work; disposable workers mutate code in isolated worktrees. Use TaskList/TaskUpdate for coordination. Name required skills explicitly in worker prompts. See CLAUDE.md and the swarm skill for the current control plane.'"
          }
        ]
      }
    ]
  }
}
SETTINGSEOF
    echo "  Created with PostToolUse, TeammateIdle, TaskCompleted, SubagentStart, SubagentStop, PreToolUse, and SessionStart hooks"
fi

# --- Print customization guide -----------------------------------------------

echo ""
echo "========================================================================"
echo " Swarm Pack — Setup Complete"
echo "========================================================================"
echo ""
if [ -d "${CLAUDE_DIR}/agents" ]; then
    AGENT_SURFACE_DIR="${CLAUDE_DIR}/agents"
    AGENT_SURFACE_LABEL=".claude/agents/"
elif [ -d "${CLAUDE_DIR}/agents/archive" ]; then
    AGENT_SURFACE_DIR="${CLAUDE_DIR}/agents/archive"
    AGENT_SURFACE_LABEL=".claude/agents/archive/"
fi
AGENT_COUNT=$(find "${AGENT_SURFACE_DIR}" -maxdepth 1 -type f -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
SKILL_COUNT=$(find "${CLAUDE_DIR}/skills" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
COMMAND_COUNT=$(ls -1 "${CLAUDE_DIR}/commands"/*.md 2>/dev/null | wc -l | tr -d ' ')
HOOK_COUNT=$(ls -1 "${CLAUDE_DIR}/hooks"/*.sh 2>/dev/null | wc -l | tr -d ' ')
echo " Installed:"
echo "   - ${AGENT_COUNT} agent definitions in ${AGENT_SURFACE_LABEL}"
echo "   - ${SKILL_COUNT} skills in .claude/skills/"
echo "   - ${COMMAND_COUNT} slash command files in .claude/commands/"
echo "   - ${HOOK_COUNT} hook scripts in .claude/hooks/"
echo "   - hooks registered in .claude/settings.json (7 event types)"
echo "   - .claude/swarm-state/  — tracked knowledge (pitfalls, slices, discoveries, findings, queue)"
echo "   - ${OPS_DIR}/           — ephemeral runtime (gitignored: handoffs, metrics, patches, salvage)"
echo "   - GitHub labels (7)"
echo ""
echo " Next steps — customize for your project:"
echo ""
echo "   1. AGENT DEFINITIONS (${AGENT_SURFACE_LABEL}*.md):"
echo "      Replace placeholder variables with your commands:"
echo ""
echo "        \$FMT_CMD          → ${FMT_CMD}"
echo "        \$FMT_CHECK_CMD    → ${FMT_CHECK_CMD}"
echo "        \$LINT_CMD          → ${LINT_CMD}"
echo "        \$TEST_CMD          → ${TEST_CMD}"
echo "        \$DEAD_CODE_CMD     → your dead code detector"
echo "        \$UNUSED_DEPS_CMD   → your unused deps checker"
echo ""
echo "   2. SCOUT FOCUS AREAS (${AGENT_SURFACE_LABEL}scout.md):"
echo "      Replace \$ERROR_SOURCE, \$TEST_GAPS, etc. with your:"
echo "        - Bug tracking / error baseline sources"
echo "        - Test coverage gap locations"
echo "        - Technical debt tracking file"
echo ""
echo "   3. DRIFT COMMANDS (${AGENT_SURFACE_LABEL}ops.md, commands/status-drift.md):"
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

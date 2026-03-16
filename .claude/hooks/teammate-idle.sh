#!/usr/bin/env bash
# Only notify on new idle transitions, not repeated idle ticks
# Uses a state file to track known-idle teammates
# Claude hook event data is passed as JSON on stdin.

STATE_DIR="/tmp/claude-swarm-idle-state"
mkdir -p "$STATE_DIR"

TEAMMATE_ID="$(jq -r '.teammate_name // "unknown"' 2>/dev/null)"
TEAMMATE_ID="${TEAMMATE_ID:-unknown}"
STATE_FILE="$STATE_DIR/$TEAMMATE_ID"

# If already tracked as idle, suppress output
if [[ -f "$STATE_FILE" ]]; then
    exit 0
fi

# First idle transition — mark and notify
touch "$STATE_FILE"
# Output nothing — the system already shows idle notifications
# Remove this file when the teammate becomes active again (via a different hook)

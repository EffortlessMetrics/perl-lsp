#!/usr/bin/env bash
# control-plane-lock.sh — advisory single-writer lock for control-plane files
#
# Protects: .claude/agents/  .claude/commands/  CLAUDE.md
# Lock file: .ops-perl-lsp/control-plane.lock  (override via CONTROL_PLANE_LOCK_FILE)
# Lock format: agent-id\ntimestamp\n
# Stale after: 30 minutes
#
# Usage:
#   control-plane-lock.sh acquire <agent-id>   — claim the lock
#   control-plane-lock.sh release <agent-id>   — release the lock
#   control-plane-lock.sh status               — show lock state
#   control-plane-lock.sh force-release        — emergency release

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

LOCK_FILE="${CONTROL_PLANE_LOCK_FILE:-$REPO_ROOT/.ops-perl-lsp/control-plane.lock}"
LOCK_TTL_SECONDS=1800  # 30 minutes

# ── Helpers ───────────────────────────────────────────────────────────────────

now_ts() { date +%s; }

# Read lock file; sets LOCK_HOLDER and LOCK_TS on success.
# Returns 1 if file doesn't exist or is malformed.
read_lock() {
    [[ -f "$LOCK_FILE" ]] || return 1
    LOCK_HOLDER=$(sed -n '1p' "$LOCK_FILE")
    LOCK_TS=$(sed -n '2p' "$LOCK_FILE")
    [[ -n "$LOCK_HOLDER" && -n "$LOCK_TS" ]] || return 1
    return 0
}

is_stale() {
    local ts="$1"
    local now
    now=$(now_ts)
    local age=$(( now - ts ))
    [[ $age -gt $LOCK_TTL_SECONDS ]]
}

write_lock() {
    local agent_id="$1"
    printf '%s\n%s\n' "$agent_id" "$(now_ts)" > "$LOCK_FILE"
}

remove_lock() {
    rm -f "$LOCK_FILE"
}

# ── Subcommands ───────────────────────────────────────────────────────────────

cmd_acquire() {
    local agent_id="${1:-}"
    if [[ -z "$agent_id" ]]; then
        echo "ERROR: acquire requires an agent-id" >&2
        echo "Usage: $0 acquire <agent-id>" >&2
        exit 2
    fi

    local LOCK_HOLDER LOCK_TS
    if read_lock; then
        if is_stale "$LOCK_TS"; then
            echo "WARN: stale lock from '$LOCK_HOLDER' (age > ${LOCK_TTL_SECONDS}s) — clearing" >&2
            remove_lock
        else
            echo "ERROR: lock held by '$LOCK_HOLDER' (acquired $(( $(now_ts) - LOCK_TS ))s ago)" >&2
            echo "Use 'status' for details, or 'force-release' for emergency override." >&2
            exit 1
        fi
    fi

    write_lock "$agent_id"
    echo "OK: lock acquired by '$agent_id'"
}

cmd_release() {
    local agent_id="${1:-}"
    if [[ -z "$agent_id" ]]; then
        echo "ERROR: release requires an agent-id" >&2
        echo "Usage: $0 release <agent-id>" >&2
        exit 2
    fi

    local LOCK_HOLDER LOCK_TS
    if ! read_lock; then
        echo "ERROR: no lock held (nothing to release)" >&2
        exit 1
    fi

    if [[ "$LOCK_HOLDER" != "$agent_id" ]]; then
        echo "ERROR: lock is held by '$LOCK_HOLDER', not '$agent_id' — cannot release" >&2
        exit 1
    fi

    remove_lock
    echo "OK: lock released by '$agent_id'"
}

cmd_status() {
    local LOCK_HOLDER LOCK_TS
    if ! read_lock; then
        echo "unlocked"
        return 0
    fi

    local now
    now=$(now_ts)
    local age=$(( now - LOCK_TS ))

    if is_stale "$LOCK_TS"; then
        echo "stale (expired): holder='$LOCK_HOLDER' age=${age}s (limit=${LOCK_TTL_SECONDS}s)"
    else
        local remaining=$(( LOCK_TTL_SECONDS - age ))
        echo "locked: holder='$LOCK_HOLDER' age=${age}s remaining=${remaining}s"
    fi
}

cmd_force_release() {
    local LOCK_HOLDER LOCK_TS
    if read_lock; then
        echo "WARN: force-releasing lock held by '$LOCK_HOLDER'" >&2
    else
        echo "INFO: no lock present (nothing to force-release)"
        return 0
    fi
    remove_lock
    echo "OK: lock force-released"
}

# ── Dispatch ──────────────────────────────────────────────────────────────────

SUBCOMMAND="${1:-}"
shift || true

case "$SUBCOMMAND" in
    acquire)       cmd_acquire "$@" ;;
    release)       cmd_release "$@" ;;
    status)        cmd_status ;;
    force-release) cmd_force_release ;;
    *)
        echo "Usage: $0 <acquire|release|status|force-release> [agent-id]" >&2
        echo ""
        echo "  acquire <agent-id>   — claim the control-plane lock"
        echo "  release <agent-id>   — release the lock (must be the holder)"
        echo "  status               — show lock state"
        echo "  force-release        — emergency release (orchestrator only)"
        echo ""
        echo "Lock file: $LOCK_FILE"
        echo "Lock TTL:  ${LOCK_TTL_SECONDS}s ($(( LOCK_TTL_SECONDS / 60 )) minutes)"
        exit 2
        ;;
esac

#!/usr/bin/env bash
# scripts/clean-tmp-targets.sh
#
# Reap orphaned agent /tmp build-target directories that exhaust disk space.
#
# These directories are created by agents via CARGO_TARGET_DIR=/tmp/agent-*-target
# (and similar patterns). Finished/zombie agents leave them behind; this script
# identifies which ones are NOT backed by a live registered git worktree and can
# be removed safely.
#
# Usage:
#   bash scripts/clean-tmp-targets.sh            # dry-run: list orphans + sizes
#   bash scripts/clean-tmp-targets.sh --prune    # actually delete orphaned dirs
#   APPLY=1 bash scripts/clean-tmp-targets.sh   # same as --prune
#
# Safety guarantees:
#   - Only touches dirs matching /tmp/agent-*-target or /tmp/wt-*-target
#   - Never removes a target dir whose path prefix matches a registered worktree
#   - Skips dirs modified in the last GRACE_MINUTES (default 5) — likely active build
#   - Dry-run by default; explicit --prune or APPLY=1 required to delete

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

# Minutes within which a directory is considered "recently touched" (active build).
GRACE_MINUTES="${GRACE_MINUTES:-5}"

# Whether to actually delete (1) or only list (0).
APPLY="${APPLY:-0}"

# Parse --prune flag.
for arg in "$@"; do
    case "$arg" in
        --prune) APPLY=1 ;;
        --dry-run) APPLY=0 ;;
        --help|-h)
            grep '^#' "$0" | sed 's/^# //' | sed 's/^#//'
            exit 0
            ;;
        *)
            echo "Unknown flag: $arg" >&2
            echo "Usage: $0 [--prune|--dry-run]" >&2
            exit 1
            ;;
    esac
done

# ── Discover live worktree paths ───────────────────────────────────────────────
#
# We cross-check /tmp target dirs against these to ensure we never remove a
# target directory that belongs to a currently registered worktree.
#
# Heuristic: given a worktree at /path/to/worktrees/agent-XXXX the expected
# CARGO_TARGET_DIR is /tmp/agent-XXXX-target (the per-branch convention from
# agent-preflight.sh and CLAUDE.md). We build a set of "safe" name fragments
# from every live worktree path and compare against candidate dir basenames.

collect_live_worktree_target_names() {
    # Try to find the repo root; fall back to CWD if not in a git repo.
    local repo_root
    repo_root="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")"

    # Collect all registered worktree paths and branches from git's porcelain
    # listing. We emit "safe" target dir basenames from two sources:
    #
    # 1. The worktree PATH basename — agent-preflight.sh names the target dir
    #    after the worktree dir (for the main checkout or early conventions).
    #    e.g. worktree path ending in "agent-XXXX" → "agent-XXXX-target"
    #
    # 2. The BRANCH NAME with '/' replaced by '-' — this is the primary
    #    convention from agent-preflight.sh:
    #    CARGO_TARGET_DIR=/tmp/agent-<branch | tr '/' '-'>-target
    #    e.g. branch "worktree-agent-XXXX" → "agent-worktree-agent-XXXX-target"
    #
    # We emit both so we cover old and new conventions without false positives.

    local wt_path="" branch_ref=""
    while IFS= read -r line; do
        case "$line" in
            "worktree "*)
                # Flush previous worktree entry.
                if [[ -n "$wt_path" ]]; then
                    local wt_base
                    wt_base="$(basename "$wt_path")"
                    echo "agent-${wt_base#agent-}-target"
                    echo "${wt_base}-target"
                fi
                wt_path="${line#worktree }"
                branch_ref=""
                ;;
            "branch "*)
                branch_ref="${line#branch }"
                # Strip refs/heads/ prefix to get the short branch name.
                local branch_name="${branch_ref#refs/heads/}"
                # Convert '/' to '-' matching agent-preflight.sh convention.
                local branch_slug="${branch_name//\//-}"
                echo "agent-${branch_slug}-target"
                ;;
        esac
    done < <(git -C "$repo_root" worktree list --porcelain 2>/dev/null || true)

    # Flush final entry.
    if [[ -n "$wt_path" ]]; then
        local wt_base
        wt_base="$(basename "$wt_path")"
        echo "agent-${wt_base#agent-}-target"
        echo "${wt_base}-target"
    fi
}

# Build a newline-separated list of "safe" /tmp dir names (those matching a
# live worktree).  We use a file so we can grep against it without subshells.
SAFE_NAMES_FILE="$(mktemp)"
trap 'rm -f "$SAFE_NAMES_FILE"' EXIT

collect_live_worktree_target_names | sort -u > "$SAFE_NAMES_FILE"

# ── Find candidate dirs in /tmp ────────────────────────────────────────────────
#
# Only these naming patterns are touched — never arbitrary paths.
declare -a PATTERNS=(
    "/tmp/agent-*-target"
    "/tmp/wt-*-target"
)

# ── Main loop ─────────────────────────────────────────────────────────────────

found_any=0
total_orphan_kb=0
orphan_count=0
skip_count=0
live_count=0

if [[ "$APPLY" -eq 1 ]]; then
    echo "=== clean-tmp-targets: PRUNE MODE ==="
else
    echo "=== clean-tmp-targets: DRY-RUN (pass --prune to delete) ==="
fi
echo ""

for pattern in "${PATTERNS[@]}"; do
    # Use glob expansion; if nothing matches, the literal glob is returned.
    for candidate in $pattern; do
        [[ -d "$candidate" ]] || continue
        found_any=1

        name="$(basename "$candidate")"

        # Safety check 1: is this a live worktree's target dir?
        if grep -qxF "$name" "$SAFE_NAMES_FILE" 2>/dev/null; then
            echo "  LIVE    $candidate  (registered worktree — skipping)"
            live_count=$((live_count + 1))
            continue
        fi

        # Safety check 2: was this dir modified recently (active build)?
        # find -mmin -N lists files modified within N minutes.
        recently_touched="$(find "$candidate" -maxdepth 1 -mmin "-${GRACE_MINUTES}" 2>/dev/null | head -1)"
        if [[ -n "$recently_touched" ]]; then
            echo "  RECENT  $candidate  (modified <${GRACE_MINUTES}min — skipping)"
            skip_count=$((skip_count + 1))
            continue
        fi

        # Measure reclaimable space.
        dir_kb=0
        dir_kb="$(du -sk "$candidate" 2>/dev/null | awk '{print $1}' || echo 0)"
        dir_human="$(du -sh "$candidate" 2>/dev/null | awk '{print $1}' || echo '?')"
        total_orphan_kb=$((total_orphan_kb + dir_kb))
        orphan_count=$((orphan_count + 1))

        if [[ "$APPLY" -eq 1 ]]; then
            echo "  REMOVE  $candidate  ($dir_human)"
            rm -rf "$candidate"
        else
            echo "  ORPHAN  $candidate  ($dir_human reclaimable)"
        fi
    done
done

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
if [[ "$found_any" -eq 0 ]]; then
    echo "No agent /tmp target dirs found (patterns: ${PATTERNS[*]})."
else
    total_human=""
    if [[ "$total_orphan_kb" -gt 0 ]]; then
        total_human=" ($(( total_orphan_kb / 1024 ))M reclaimable)"
    fi

    echo "Live worktree targets skipped:  $live_count"
    echo "Recently-modified dirs skipped: $skip_count"
    if [[ "$APPLY" -eq 1 ]]; then
        echo "Orphaned dirs removed:          $orphan_count${total_human}"
    else
        echo "Orphaned dirs identified:       $orphan_count${total_human}"
        if [[ "$orphan_count" -gt 0 ]]; then
            echo ""
            echo "Re-run with --prune to delete, or: APPLY=1 bash scripts/clean-tmp-targets.sh"
        fi
    fi
fi

#!/usr/bin/env bash
# check_release_history.sh — Detect drift between git tags, release notes, and the release ledger.
#
# Exits 0 if no drift is detected, exits 1 with descriptive messages if drift is found.
#
# Exemptions:
#   - Prerelease tags (v*-rc*) are ignored entirely
#   - (CL) entries in RELEASE_HISTORY.md have no tag and are scope markers (not releases)
#   - Grandfathered gaps: tags in RELEASE_HISTORY.md without a [n-X.Y.Z]: link
#     (e.g., v0.7.2, v0.7.3, v0.8.0, v0.8.2, v0.5.0, v0.1.0-pest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ── Helpers ───────────────────────────────────────────────────────────────────

# Print error message and set error flag
error() {
    echo "ERROR: $1" >&2
    DRIFT_FOUND=1
}

# Print warning message (does not cause failure)
warn() {
    echo "WARN: $1" >&2
}

# ── Collect non-RC tags ───────────────────────────────────────────────────────

# Get all v* tags, strip "v" prefix, exclude prerelease tags (v*-rc*)
# shellcheck disable=SC2046
mapfile -t ALL_TAGS < <(git tag --list 'v*' | sed 's/^v//' | grep -v 'rc')

# ── Parse RELEASE_HISTORY.md ──────────────────────────────────────────────────

# Grandfathered versions: tags with no docs/releases/v<X.Y.Z>.md that appear
# in RELEASE_HISTORY.md without a [n-X.Y.Z]: link. These are older releases
# that never had notes files - they are grandfathered.

# Collect all grandfathered versions (tags that have no notes file but are in RELEASE_HISTORY)
declare -A GRANDFATHERED_VERSIONS
for tag in "${ALL_TAGS[@]}"; do
    notes_file="docs/releases/v${tag}.md"
    if [[ ! -f "$notes_file" ]]; then
        # No notes file — check if it's in RELEASE_HISTORY (older entries use "0.7.2" not "[0.7.2]")
        if grep -q "${tag}" RELEASE_HISTORY.md 2>/dev/null; then
            # Check if there's a markdown link for notes file
            if ! grep -q "\[n-${tag}\]:" RELEASE_HISTORY.md 2>/dev/null; then
                GRANDFATHERED_VERSIONS["$tag"]=1
                warn "Grandfathered gap: v${tag} has no notes file (expected — see RELEASE_HISTORY.md)"
            fi
        fi
    fi
done

# Collect (CL) versions — entries marked as CHANGELOG-only (no tag exists)
# These are scope markers like v0.9.0, v0.10.0, v0.8.8
# In RELEASE_HISTORY.md they show "—" in the Tag column and "(CL)" in the Released column
declare -A CL_ONLY_VERSIONS
while IFS= read -r line; do
    # Extract version from [X.Y.Z] link pattern
    if [[ $line =~ \[([0-9]+\.[0-9]+\.[0-9]+[-.]?[0-9]*)\] ]]; then
        ver="${BASH_REMATCH[1]}"
        CL_ONLY_VERSIONS["$ver"]=1
    fi
done < <(grep '(CL)' RELEASE_HISTORY.md 2>/dev/null || true)

# ── Check 1: Each non-RC tag must have release notes file ───────────────────

DRIFT_FOUND=0

for tag in "${ALL_TAGS[@]}"; do
    # Skip (CL) entries — they have no tag by definition
    if [[ -n "${CL_ONLY_VERSIONS[$tag]:-}" ]]; then
        continue
    fi

    # For non-(CL) tags, check notes file exists
    notes_file="docs/releases/v${tag}.md"
    if [[ ! -f "$notes_file" ]]; then
        # Check if it's a grandfathered gap
        if [[ -n "${GRANDFATHERED_VERSIONS[$tag]:-}" ]]; then
            # Already warned above, skip
            continue
        fi
        error "Missing release notes: docs/releases/v${tag}.md"
    fi
done

# ── Check 2: Each non-RC tag must have RELEASE_HISTORY.md entry ─────────────

for tag in "${ALL_TAGS[@]}"; do
    # Skip (CL) entries — they don't have tags
    if [[ -n "${CL_ONLY_VERSIONS[$tag]:-}" ]]; then
        continue
    fi

    # Check RELEASE_HISTORY.md contains this version
    # Use plain grep since older versions appear as "0.7.2" (no brackets) in the table
    if ! grep -q "${tag}" RELEASE_HISTORY.md 2>/dev/null; then
        error "Missing RELEASE_HISTORY.md entry for ${tag}"
    fi
done

# ── Check 3: Newest tag must be in CHANGELOG.md ───────────────────────────────

# Find the newest (highest) tag by semantic version sort
NEWEST_TAG=""
if [[ ${#ALL_TAGS[@]} -gt 0 ]]; then
    NEWEST_TAG=$(printf '%s\n' "${ALL_TAGS[@]}" | sort -V | tail -1)
fi

if [[ -z "$NEWEST_TAG" ]]; then
    warn "No non-RC tags found"
else
    # Check CHANGELOG.md contains ## [X.Y.Z] for newest tag
    if ! grep -q "## \[${NEWEST_TAG}\]" CHANGELOG.md 2>/dev/null; then
        error "Newest tag v${NEWEST_TAG} not found in CHANGELOG.md"
    fi
fi

# ── Exit ──────────────────────────────────────────────────────────────────────

if [[ "$DRIFT_FOUND" -eq 1 ]]; then
    echo "Release history drift detected." >&2
    exit 1
fi

echo "Release history drift check passed."
exit 0

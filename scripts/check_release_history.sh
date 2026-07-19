#!/usr/bin/env bash
# check_release_history.sh — Detect drift between git tags, release notes, and the release ledger.
#
# Exits 0 if no drift is detected, exits 1 with descriptive messages if drift is found.
#
# Exemptions:
#   - Prerelease tags (v*-rc*) are ignored entirely
#   - (CL) entries in RELEASE_HISTORY.md have no tag and are scope markers (not releases)
#   - Grandfathered gaps: tags with "—" in the Notes file column of RELEASE_HISTORY.md
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
# sort_tags — Sort tags by semantic version (used by NEWEST_TAG calculation)
sort_tags() {
    printf '%s\n' "${ALL_TAGS[@]}"
}

# ── Parse RELEASE_HISTORY.md ──────────────────────────────────────────────────

# Grandfathered versions: have "—" in the Notes file column (col 10 in the table)
# These are older releases that never had notes files - they are grandfathered.
# We identify them by finding rows where the Tag column has a real tag (not "—")
# but the Notes file column (last column) has "—".
#
# The table format (markdown table):
# | Version | Tag | GitHub Release | Released | Tag commit | Compare | Assets | crates.io | VS Code Marketplace | Notes file |
# We extract column 1 (Version) for rows where:
#   - Column 2 (Tag) is not "—" (has a real tag)
#   - Column 10 (Notes file) is "—" (no notes file)
#
# We parse the links section at the bottom of RELEASE_HISTORY.md:
# [n-0.7.2]: docs/releases/v0.7.2.md  (or absent if no notes file)
#
# The simplest approach: for each tag that has no docs/releases/v<X.Y.Z>.md,
# if the tag appears in RELEASE_HISTORY with a "—" in the notes column, it's grandfathered.

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

# Keep the tag-provenance manifest as a persistent release-history gate. This
# uses the same full-history checkout and fetched tags as the checks below.
if ! python3 scripts/check_release_tag_provenance.py --verify-git --repo-root "$REPO_ROOT"; then
    error "Release-tag provenance drift check failed"
fi

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

    newest_notes_file="docs/releases/v${NEWEST_TAG}.md"
    if [[ -f "$newest_notes_file" ]] && grep -Eq 'unknown-linux-(gnu|musl)' "$newest_notes_file"; then
        if ! grep -q 'Which file should I download?' "$newest_notes_file" &&
           ! grep -q 'docs/how-to/INSTALLATION.md' "$newest_notes_file" &&
           ! grep -q 'INSTALLATION.md' "$newest_notes_file"; then
            error "Newest release notes with Linux GNU/musl assets must explain which file to download: ${newest_notes_file}"
        fi
    fi
fi

# ── Check 4: Active install docs must not advertise retired Homebrew commands ─

for forbidden_homebrew_pattern in \
    'brew install perl''-lsp' \
    'brew tap effortlesssteven''/tap' \
    'brew tap tree-sitter-perl''/tap'
do
    forbidden_output="$(
        git grep -n -F "$forbidden_homebrew_pattern" -- \
            ':!**/target/**' \
            ':!docs/reference/archive/**' \
            ':!docs/issues/**' \
            ':!scripts/check_release_history.sh' \
            ':!xtask/src/tasks/install_surface_check.rs' \
            || true
    )"
    if [[ -n "$forbidden_output" ]]; then
        while IFS= read -r line; do
            error "Forbidden retired Homebrew command in active docs: ${line}"
        done <<< "$forbidden_output"
    fi
done

# ── Check 5: Active install surfaces stay aligned with current package names ─

run_install_surface_check() {
    xtask_binary_is_fresh() {
        local bin="$1"
        [[ -x "$bin" ]] || return 1
        ! find Cargo.toml Cargo.lock xtask/Cargo.toml xtask/src -type f -newer "$bin" -print -quit | grep -q .
    }

    local candidate
    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        for candidate in "$CARGO_TARGET_DIR/debug/xtask" "$CARGO_TARGET_DIR/debug/xtask.exe"; do
            if xtask_binary_is_fresh "$candidate"; then
                "$candidate" install-surface-check
                return
            fi
        done
    fi

    for candidate in target/debug/xtask target/debug/xtask.exe; do
        if xtask_binary_is_fresh "$candidate"; then
            "$candidate" install-surface-check
            return
        fi
    done

    if cargo metadata --no-deps --format-version 1 >/dev/null 2>&1; then
        cargo xtask install-surface-check
        return
    fi

    if [[ -x target/debug/xtask.exe ]]; then
        target/debug/xtask.exe install-surface-check
        return
    fi

    if [[ -x target/debug/xtask ]]; then
        target/debug/xtask install-surface-check
        return
    fi

    cargo xtask install-surface-check
}

if ! run_install_surface_check; then
    error "Install surface drift check failed"
fi

# ── Check 6: Verified channel actuals must not regress ─────────────────────────

if ! python3 scripts/check_release_channel_actuals.py; then
    error "Release-channel actuals drift check failed"
fi

# ── Check 7: Audited container actuals must not regress ───────────────────────

if ! python3 scripts/check_release_container_actuals.py; then
    error "Release-container actuals drift check failed"
fi

if ! python3 scripts/tests/test_release_container_actuals.py; then
    error "Release-container actuals validator tests failed"
fi

# ── Exit ──────────────────────────────────────────────────────────────────────

if [[ "$DRIFT_FOUND" -eq 1 ]]; then
    echo "Release history drift detected." >&2
    exit 1
fi

echo "Release history drift check passed."
exit 0

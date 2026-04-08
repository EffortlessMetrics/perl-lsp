#!/usr/bin/env bash
# publish-new-crates-manually.sh
#
# Manual helper for first-publishing NEW crates to crates.io one at a time.
#
# Usage:
#   bash scripts/publish-new-crates-manually.sh
#   DRY_RUN=true bash scripts/publish-new-crates-manually.sh
#
# Context:
#   crates.io imposes a SEPARATE account-level rate limit for first-time crate
#   creation (distinct from the update rate limit). Burst capacity is 5, refilling
#   at 1 token every 10 minutes. If you exhaust the burst, every subsequent
#   attempt fails immediately with:
#     "You have published too many new crates in a short period of time"
#   This script serialises the 4 new crates with a 10-minute pause between
#   each publish to stay comfortably under the refill rate.
#
# Environment variables:
#   DRY_RUN=true    Print what would be done; do not actually publish.
#   SLEEP_SECONDS   Override the inter-publish sleep (default: 600). Set to 0
#                   for testing the dry-run logic; never 0 in production.
#   CARGO_REGISTRY_TOKEN  Must be set (or cargo must have a token in config).

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
DRY_RUN="${DRY_RUN:-false}"
SLEEP_SECONDS="${SLEEP_SECONDS:-600}"

# These are the 4 crates that are new to crates.io for v0.12.2.
# They appear in the workspace publish allowlist in this topological order:
#
#   perl-test-generators       — Tier 1  (leaf: only external deps)
#   tree-sitter-perl-c         — Tier 1  (leaf: only external deps)
#   tree-sitter-perl-rs        — Tier 2+ (depends on perl-parser-core + perl-ast,
#                                          both of which are existing crates)
#   perl-workspace-index-monitoring — Tier 2+ (leaf among new crates: only
#                                          external dep parking_lot; dev-dep on
#                                          perl-tdd-support is an existing crate)
#
# None of these 4 crates depend on each other, so the order is flexible.
# This order matches the workspace allowlist topological sort.
NEW_CRATES=(
    "perl-test-generators"
    "tree-sitter-perl-c"
    "tree-sitter-perl-rs"
    "perl-workspace-index-monitoring"
)

TOTAL=${#NEW_CRATES[@]}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
log()  { echo "[$(date '+%H:%M:%S')] $*"; }
info() { log "INFO  $*"; }
warn() { log "WARN  $*"; }
err()  { log "ERROR $*" >&2; }

# Compute the sparse-index path for a crate name.
# See https://doc.rust-lang.org/cargo/reference/registry-index.html
sparse_index_path() {
    local name_lc
    name_lc=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
    local len=${#name_lc}
    case "$len" in
        1) printf '1/%s' "$name_lc" ;;
        2) printf '2/%s' "$name_lc" ;;
        3) printf '3/%s/%s' "${name_lc:0:1}" "$name_lc" ;;
        *) printf '%s/%s/%s' "${name_lc:0:2}" "${name_lc:2:2}" "$name_lc" ;;
    esac
}

# Returns 0 if <crate>@<version> is present (non-yanked) in the sparse index.
check_sparse_index() {
    local name="$1"
    local version="$2"
    local path
    path=$(sparse_index_path "$name")
    local url="https://index.crates.io/${path}"
    local body
    body=$(curl -fsSL --max-time 15 "$url" 2>/dev/null) || return 1
    printf '%s\n' "$body" | VERS="$version" python3 -c '
import json, os, sys
target = os.environ["VERS"]
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        entry = json.loads(line)
    except Exception:
        continue
    if entry.get("vers") == target and not entry.get("yanked", False):
        sys.exit(0)
sys.exit(1)
'
}

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
info "=== Manual New-Crate Publisher ==="
info "Mode: ${DRY_RUN}"
info "Crates to publish: ${TOTAL}"
info "Inter-publish sleep: ${SLEEP_SECONDS}s"
echo ""

# Resolve the version from workspace Cargo.toml
WORKSPACE_VERSION=$(python3 -c '
import re, sys
text = open("Cargo.toml").read()
m = re.search(r"^version\s*=\s*\"([^\"]+)\"", text, re.MULTILINE)
if m:
    print(m.group(1))
else:
    sys.exit(1)
')
info "Workspace version: ${WORKSPACE_VERSION}"
echo ""

if [[ "$DRY_RUN" != "true" ]] && [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    # Cargo may have a token in ~/.cargo/credentials.toml — that is fine.
    # We only warn; cargo publish itself will error if no token is found.
    warn "CARGO_REGISTRY_TOKEN is not set. cargo publish will use credentials from ~/.cargo/credentials.toml"
fi

# Build the manifest map: crate_name -> manifest_path
MANIFEST_MAP="$(mktemp)"
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
ws = set(meta["workspace_members"])
for pkg in meta["packages"]:
    if pkg["id"] in ws:
        print("{} {}".format(pkg["name"], pkg["manifest_path"]))
' > "$MANIFEST_MAP"

# The dev-dep strip script (same logic as publish-crates.yml)
STRIP_SCRIPT="$(mktemp --suffix=.py)"
cat > "$STRIP_SCRIPT" << 'STRIP_EOF'
import re, sys, pathlib
path = pathlib.Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
new_text = re.sub(
    r"^\[dev-dependencies\].*?(?=^\[|\Z)",
    "",
    text,
    flags=re.MULTILINE | re.DOTALL,
)
path.write_text(new_text, encoding="utf-8")
STRIP_EOF

# ---------------------------------------------------------------------------
# Main publish loop
# ---------------------------------------------------------------------------
PUBLISHED=0
SKIPPED=0

for i in "${!NEW_CRATES[@]}"; do
    CRATE="${NEW_CRATES[$i]}"
    IDX=$((i + 1))

    echo ""
    info "--- Publishing ${IDX}/${TOTAL}: ${CRATE} ---"

    # Determine version (all workspace members share the workspace version)
    VERSION="${WORKSPACE_VERSION}"

    # Check sparse index: if already published, skip
    if [[ "$DRY_RUN" != "true" ]]; then
        if check_sparse_index "$CRATE" "$VERSION"; then
            info "${CRATE}@${VERSION} already present in sparse index — skipping"
            SKIPPED=$((SKIPPED + 1))
            continue
        fi
    fi

    # Locate manifest path
    MANIFEST_PATH="$(grep "^${CRATE} " "$MANIFEST_MAP" | awk '{print $2}')"
    if [[ -z "$MANIFEST_PATH" ]]; then
        err "Could not find manifest path for ${CRATE}"
        exit 1
    fi
    info "Manifest: ${MANIFEST_PATH}"

    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would strip [dev-dependencies] from ${MANIFEST_PATH}"
        info "[DRY RUN] Would run: cargo publish -p ${CRATE} --no-verify --allow-dirty"
        info "[DRY RUN] Would wait for sparse index visibility"
        info "[DRY RUN] Would restore ${MANIFEST_PATH}"
        if [[ $IDX -lt $TOTAL ]]; then
            info "[DRY RUN] Would sleep ${SLEEP_SECONDS}s before next crate"
        fi
        PUBLISHED=$((PUBLISHED + 1))
        continue
    fi

    # Back up manifest and register restore trap
    MANIFEST_BACKUP="$(mktemp)"
    cp "$MANIFEST_PATH" "$MANIFEST_BACKUP"
    # shellcheck disable=SC2064
    trap "cp '$MANIFEST_BACKUP' '$MANIFEST_PATH'; rm -f '$MANIFEST_BACKUP'" EXIT

    # Strip [dev-dependencies]
    info "Stripping [dev-dependencies] from manifest before publish"
    python3 "$STRIP_SCRIPT" "$MANIFEST_PATH"

    # Publish with up to 3 attempts
    PUBLISH_OK=false
    for attempt in 1 2 3; do
        info "Attempt ${attempt}/3: cargo publish -p ${CRATE} --no-verify --allow-dirty"
        PUBLISH_LOG="$(mktemp)"
        PUBLISH_EXIT=0
        cargo publish -p "$CRATE" --no-verify --allow-dirty 2>&1 | tee "$PUBLISH_LOG" || PUBLISH_EXIT=${PIPESTATUS[0]}

        # Detect rate limit (new-crate creation limit)
        if grep -qE "429|Too Many Requests|too many new crates" "$PUBLISH_LOG"; then
            err "Hit crates.io new-crate rate limit (429)."
            err "Rate limit: burst=5, refill=1 per 10 minutes."
            err "Wait at least 10 minutes before retrying, or wait until tomorrow."
            err "See docs/reference/MANUAL_PUBLISH_NEW_CRATES.md for context."
            rm -f "$PUBLISH_LOG"
            cp "$MANIFEST_BACKUP" "$MANIFEST_PATH"
            rm -f "$MANIFEST_BACKUP"
            trap - EXIT
            exit 4
        fi

        # Detect "already exists" (benign)
        if grep -q "already exists on crates.io index" "$PUBLISH_LOG"; then
            warn "${CRATE}@${VERSION} already exists — skipping"
            rm -f "$PUBLISH_LOG"
            PUBLISH_OK=true
            break
        fi

        rm -f "$PUBLISH_LOG"

        if [[ "$PUBLISH_EXIT" != "0" ]]; then
            warn "Publish attempt ${attempt} failed (exit ${PUBLISH_EXIT}), waiting 30s before retry..."
            sleep 30
            continue
        fi

        # cargo publish succeeded — wait for sparse index visibility
        info "Waiting for ${CRATE}@${VERSION} to appear in sparse index..."
        INDEXED=false
        ELAPSED=0
        for delta in 5 10 30 45; do
            sleep "$delta"
            ELAPSED=$((ELAPSED + delta))
            if check_sparse_index "$CRATE" "$VERSION"; then
                info "${CRATE}@${VERSION} indexed after ${ELAPSED}s"
                INDEXED=true
                break
            fi
            info "  ${ELAPSED}s elapsed, not yet visible in sparse index..."
        done

        if [[ "$INDEXED" == "true" ]]; then
            PUBLISH_OK=true
            break
        fi

        warn "${CRATE}@${VERSION} not visible in sparse index after 90s on attempt ${attempt}/3"
    done

    # Restore manifest
    cp "$MANIFEST_BACKUP" "$MANIFEST_PATH"
    rm -f "$MANIFEST_BACKUP"
    trap - EXIT

    if [[ "$PUBLISH_OK" != "true" ]]; then
        err "Failed to confirm ${CRATE}@${VERSION} in sparse index after 3 attempts"
        err "Check: https://index.crates.io/$(sparse_index_path "$CRATE")"
        exit 1
    fi

    PUBLISHED=$((PUBLISHED + 1))
    info "${CRATE}@${VERSION} published successfully"

    # Inter-publish sleep — only between crates, not after the last one
    if [[ $IDX -lt $TOTAL ]]; then
        info "Sleeping ${SLEEP_SECONDS}s before next crate (crates.io new-crate rate limit: 1 per 10 min)..."
        sleep "$SLEEP_SECONDS"
    fi
done

echo ""
info "=== Done: published=${PUBLISHED}, skipped=${SKIPPED}, total=${TOTAL} ==="

if [[ "$DRY_RUN" == "true" ]]; then
    info "Dry run complete. Remove DRY_RUN=true to actually publish."
fi

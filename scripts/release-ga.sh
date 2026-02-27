#!/usr/bin/env bash
set -euo pipefail

# ======================================================
# perl-lsp v0.8.3 GA release script (bash 4+, macOS/Linux)
# ======================================================
# What it does:
#   1) Bump versions across Cargo.toml files to match --version
#   2) Run unit + property tests, clippy (D warnings), optional deep props
#   3) Tag & push (unless --no-tag)
#   4) Wait for GitHub Release artifacts (from cargo-dist CI) to appear
#   5) Download artifacts, compute SHA256, produce:
#        - Homebrew formula (stdout template)
#        - VS Code asset map (stdout JSON)
#   6) Lift CHANGELOG notes into the GitHub Release body (if present)
#
# Requires: git, cargo, jq, gh, sed, awk, sha256sum (Linux) or shasum (macOS)
#
# Usage (dry-run first):
#   scripts/release-ga.sh \
#     --version v0.8.3 \
#     --owner EffortlessMetrics \
#     --repo perl-lsp \
#     --brew-tap effortlesssteven/tools \
#     --dry-run
#
# Real run:
#   scripts/release-ga.sh \
#     --version v0.8.3 \
#     --owner EffortlessMetrics \
#     --repo perl-lsp \
#     --brew-tap effortlesssteven/tools
#
# Notes:
# - Property test counts: PROPTEST_CASES fast(default 64)/deep(default 256)
#   export PROPTEST_CASES to override, or pass env below.
# - Assumes cargo-dist release job creates the GH release + artifacts.

# ----------- Args / defaults
VERSION=""
OWNER="${OWNER:-}"
REPO="${REPO:-}"
BREW_TAP="${BREW_TAP:-}"
DRY_RUN=0
NO_TAG=0
SKIP_TESTS=0
BENCH_GUARD_PCT="${BENCH_GUARD_PCT:-10}"   # (keep for future use)
ARTIFACT_PREFIX="${ARTIFACT_PREFIX:-perl-lsp}"
RELEASE_WAIT_SECS="${RELEASE_WAIT_SECS:-900}" # 15m default
NO_WAIT=0
PROP_FAST="${PROP_FAST:-64}"
PROP_DEEP="${PROP_DEEP:-256}"
MAIN_BRANCH="${MAIN_BRANCH:-main}"

log()  { printf "\033[1;34m[release]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[warn]\033[0m %s\n" "$*"; }
die()  { printf "\033[1;31m[error]\033[0m %s\n" "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "Missing dependency: $1"; }

usage() {
  cat <<USAGE
Usage: scripts/release-ga.sh --version vX.Y.Z --owner <org> --repo <name> [options]
Options:
  --brew-tap <tap>     Homebrew tap like org/tools
  --dry-run            Do everything except push/edit remote; also skips wait
  --no-tag             Do not create/push a git tag
  --no-wait            Do not wait for GitHub release artifacts
  --skip-tests         Skip tests and clippy
  --help               This help text
Env:
  ARTIFACT_PREFIX      Artifact base name (default: perl-lsp)
  RELEASE_WAIT_SECS    Max seconds to wait for release assets (default: 900)
USAGE
}

sed_inplace() {
  # cross-platform in-place sed
  if [[ "$OSTYPE" == darwin* ]]; then sed -i '' "$@"; else sed -i "$@"; fi
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else die "No sha256sum/shasum found"
  fi
}

# ----------- Argparse
while (( "$#" )); do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --owner) OWNER="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    --brew-tap) BREW_TAP="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --no-tag) NO_TAG=1; shift ;;
    --skip-tests) SKIP_TESTS=1; shift ;;
    --no-wait) NO_WAIT=1; shift ;;
    --help) usage; exit 0 ;;
    *) die "Unknown arg: $1" ;;
  esac
done

[[ -n "$VERSION" ]] || die "--version required (e.g. v0.8.3)"
[[ -n "$OWNER"   ]] || die "--owner required"
[[ -n "$REPO"    ]] || die "--repo required"

PLAIN_VERSION="${VERSION#v}" # strip 'v' for Cargo.toml

# ----------- Preflight
need git; need cargo; need jq; need gh
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "Not a git repo"
gh auth status >/dev/null 2>&1 || die "GitHub CLI not authenticated (run: gh auth login)"
[[ -z "$(git status --porcelain)" ]] || die "Working tree not clean (staged/unstaged/untracked files)"
current_branch="$(git rev-parse --abbrev-ref HEAD)"

if [[ "$current_branch" != "$MAIN_BRANCH" ]]; then
  warn "Not on $MAIN_BRANCH (on: $current_branch). Continuing anyway."
fi

log "Releasing $OWNER/$REPO $VERSION (plain: $PLAIN_VERSION)"
[[ $DRY_RUN -eq 1 ]] && warn "DRY RUN mode enabled"

# ----------- 1) version bump
bump_version() {
  log "Bumping versions to $PLAIN_VERSION"
  mapfile -t tomls < <(git ls-files | grep -E 'Cargo\.toml$')
  for f in "${tomls[@]}"; do
    # Only touch lines that *start* with 'version =', allowing leading spaces.
    sed_inplace -E 's/^([[:space:]]*version[[:space:]]*=[[:space:]]*)".*/\1"'"$PLAIN_VERSION"'"/' "$f"
  done
  cargo update
}

# ----------- 2) tests & clippy
run_tests() {
  [[ $SKIP_TESTS -eq 1 ]] && { warn "Skipping tests (--skip-tests)"; return; }
  log "Unit tests"
  cargo test --all

  log "Property tests (fast)"
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_invariants  -- --nocapture
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_qw          -- --nocapture
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_quote_like  -- --nocapture
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_whitespace  -- --nocapture
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_whitespace_idempotence -- --nocapture
  env PROPTEST_CASES="${PROP_FAST}" cargo test -p perl-parser --test prop_deletion    -- --nocapture

  log "Clippy (deny warnings)"
  cargo clippy --all --all-targets -- -D warnings

  # Optional deep props (CI typically runs these)
  log "Property tests (deep) — optional locally"
  env PROPTEST_CASES="${PROP_DEEP}" cargo test -p perl-parser --test prop_quote_like  -- --nocapture || warn "Deep prop failed locally; CI will catch."
}

# ----------- 3) tag & push
tag_and_push() {
  if [[ $NO_TAG -eq 1 ]]; then warn "--no-tag: skipping tag"; return; fi
  log "Commit + tag"
  git add -A
  git commit -m "chore: release $VERSION"
  git tag -a "$VERSION" -m "Release $VERSION"

  if [[ $DRY_RUN -eq 1 ]]; then
    warn "(dry-run) Skipping pushes"
  else
    git push origin "$current_branch"
    git push origin "$VERSION"
  fi
}

# ----------- 4) wait for GH release artifacts
wait_for_release() {
  if [[ $DRY_RUN -eq 1 || $NO_TAG -eq 1 || $NO_WAIT -eq 1 ]]; then
    warn "Skipping wait for release assets (dry-run/no-tag/no-wait)."
    return
  fi
  log "Waiting for GH release artifacts: $VERSION"
  for ((i=0; i<RELEASE_WAIT_SECS; i+=5)); do
    if gh release view "$VERSION" -R "$OWNER/$REPO" >/dev/null 2>&1; then
      assets="$(gh release view "$VERSION" -R "$OWNER/$REPO" --json assets | jq '.assets | length')"
      [[ "$assets" -gt 0 ]] && { log "Release found with $assets assets"; return; }
    fi
    sleep 5
  done
  die "Timed out waiting for release assets"
}

# ----------- 5) download & compute checksums
download_and_checksums() {
  [[ $DRY_RUN -eq 1 ]] && { warn "Skipping download (dry-run mode)"; return; }
  
  log "Downloading release assets"
  TMP="target/release-$VERSION"
  mkdir -p "$TMP"
  gh release download "$VERSION" -R "$OWNER/$REPO" -D "$TMP" --clobber

  log "SHA256"
  mapfile -t files < <(find "$TMP" -type f \( -name "*.tar.gz" -o -name "*.zip" \) | sort)
  CHECKSUMS_JSON="$TMP/checksums.json"
  echo "{}" > "$CHECKSUMS_JSON"
  for f in "${files[@]}"; do
    b="$(basename "$f")"
    sum="$(sha256 "$f")"
    jq --arg name "$b" --arg sum "$sum" '. + {($name): $sum}' "$CHECKSUMS_JSON" > "$CHECKSUMS_JSON.tmp"
    mv "$CHECKSUMS_JSON.tmp" "$CHECKSUMS_JSON"
    printf "  %s  %s\n" "$sum" "$b"
  done
  log "→ $CHECKSUMS_JSON"
}

# ----------- 6) print Brew formula template (stdout)
emit_brew_formula() {
  [[ -z "${BREW_TAP}" ]] && { warn "No --brew-tap provided; skipping Brew output"; return; }
  log "Homebrew formula (copy/paste to your tap; fill SHA placeholders):"
  cat <<'RUBY' | \
    sed -e "s|__OWNER__|$OWNER|g" \
        -e "s|__REPO__|$REPO|g" \
        -e "s|__VER__|$VERSION|g" \
        -e "s|__PREFIX__|$ARTIFACT_PREFIX|g"
class PerlLsp < Formula
  desc "Perl language server with 100% edge case coverage"
  homepage "https://github.com/__OWNER__/__REPO__"
  version "__VER__"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/__OWNER__/__REPO__/releases/download/__VER__/__PREFIX__-__VER__-aarch64-apple-darwin.tar.gz"
      sha256 "<SHA256_MAC_ARM64>"
    end
    on_intel do
      url "https://github.com/__OWNER__/__REPO__/releases/download/__VER__/__PREFIX__-__VER__-x86_64-apple-darwin.tar.gz"
      sha256 "<SHA256_MAC_X64>"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/__OWNER__/__REPO__/releases/download/__VER__/__PREFIX__-__VER__-aarch64-unknown-linux-musl.tar.gz"
      sha256 "<SHA256_LINUX_ARM64>"
    end
    on_intel do
      url "https://github.com/__OWNER__/__REPO__/releases/download/__VER__/__PREFIX__-__VER__-x86_64-unknown-linux-musl.tar.gz"
      sha256 "<SHA256_LINUX_X64>"
    end
  end

  def install
    bin.install "perl-lsp"
  end

  test do
    assert_match "perl-lsp", shell_output("#{bin}/perl-lsp --version")
  end
end
RUBY
  warn "Fill SHA256 placeholders using target/release-$VERSION/checksums.json"
}

# ----------- 7) print VS Code asset map (stdout)
emit_vscode_asset_map() {
  log "VS Code asset map (paste into extension/asset map + pair with checksums):"
  cat <<JSON
{
  "v": "$VERSION",
  "linux-x64":   "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-x86_64-unknown-linux-musl.tar.gz",
  "linux-arm64": "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-aarch64-unknown-linux-musl.tar.gz",
  "macos-x64":   "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-x86_64-apple-darwin.tar.gz",
  "macos-arm64": "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-aarch64-apple-darwin.tar.gz",
  "win-x64":     "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-x86_64-pc-windows-msvc.zip",
  "win-arm64":   "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$ARTIFACT_PREFIX-$VERSION-aarch64-pc-windows-msvc.zip"
}
JSON
}

# ----------- 8) update GH release notes from CHANGELOG
update_release_notes() {
  if [[ -f CHANGELOG.md ]]; then
    log "Updating release notes from CHANGELOG.md section [$VERSION]"
    body="$(awk -v ver="## [$VERSION]" '
      $0==ver {found=1; next}
      found && /^## \[/ {exit}
      found {print}
    ' CHANGELOG.md || true)"
    if [[ -n "$body" ]]; then
      [[ $DRY_RUN -eq 1 ]] && { warn "(dry-run) skipping gh release edit"; return; }
      gh release edit "$VERSION" -R "$OWNER/$REPO" --notes "$body" || warn "Could not update GH release notes"
    else
      warn "No matching section in CHANGELOG.md for $VERSION"
    fi
  else
    warn "CHANGELOG.md not found"
  fi
}

# ----------- orchestrate
bump_version
run_tests
tag_and_push
wait_for_release
download_and_checksums
emit_brew_formula
emit_vscode_asset_map
update_release_notes

log "Done. Next steps:"
echo "  1) Replace SHA placeholders in Brew formula with values from target/release-$VERSION/checksums.json"
echo "  2) Commit formula to tap ($BREW_TAP): Formula/perl-lsp.rb, then: brew tap $BREW_TAP && brew install perl-lsp"
echo "  3) Paste VS Code asset map into extension; add checksum verification"
echo "  4) Verify installers (bash/ps1), update README/docs, announce 🚀"
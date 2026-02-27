#!/usr/bin/env bash
set -euo pipefail

# Injects sha256 into a Homebrew formula + emits a VS Code asset map with sha256
# Usage:
#   scripts/inject-sha-assets.sh \
#     --version v0.8.3 \
#     --owner EffortlessMetrics \
#     --repo perl-lsp \
#     --prefix perl-lsp \
#     --checksums target/release-v0.8.3/checksums.json \
#     --brew-out Formula/perl-lsp.rb \
#     --asset-map-out extension/assets.v0.8.3.json
#
# Notes: expects cargo-dist artifact names:
#   <prefix>-<ver>-{x86_64,aarch64}-{apple-darwin,unknown-linux-musl,pc-windows-msvc}.{tar.gz,zip}

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
need jq

VERSION=""
OWNER=""
REPO=""
PREFIX=""
CHECKSUMS=""
BREW_OUT=""
ASSET_MAP_OUT=""

while (( "$#" )); do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --owner) OWNER="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --checksums) CHECKSUMS="$2"; shift 2 ;;
    --brew-out) BREW_OUT="$2"; shift 2 ;;
    --asset-map-out) ASSET_MAP_OUT="$2"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

[[ -n "$VERSION" && -n "$OWNER" && -n "$REPO" && -n "$PREFIX" && -n "$CHECKSUMS" ]] \
  || { echo "missing required args"; exit 1; }
[[ -f "$CHECKSUMS" ]] || { echo "checksums not found: $CHECKSUMS" >&2; exit 1; }

# Build filename -> sha map
declare -A SHA
while IFS=$'\t' read -r name sum; do
  SHA["$name"]="$sum"
done < <(jq -r 'to_entries[] | "\(.key)\t\(.value)"' "$CHECKSUMS")

fn() { echo "$PREFIX-$VERSION-$1"; }
sha() { local k; k="$(fn "$1")"; echo "${SHA[$k]:-}"; }

MAC_ARM="aarch64-apple-darwin.tar.gz"
MAC_X64="x86_64-apple-darwin.tar.gz"
LIN_ARM="aarch64-unknown-linux-musl.tar.gz"
LIN_X64="x86_64-unknown-linux-musl.tar.gz"
WIN_X64="x86_64-pc-windows-msvc.zip"
WIN_ARM="aarch64-pc-windows-msvc.zip"

# Validate presence
for key in "$MAC_ARM" "$MAC_X64" "$LIN_ARM" "$LIN_X64" "$WIN_X64" "$WIN_ARM"; do
  [[ -n "$(sha "$key")" ]] || { echo "missing checksum for $(fn "$key")" >&2; exit 2; }
done

BREW_FORMULA=$(cat <<RUBY
class PerlLsp < Formula
  desc "Perl language server"
  homepage "https://github.com/$OWNER/$REPO"
  version "$VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$PREFIX-$VERSION-$MAC_ARM"
      sha256 "$(sha "$MAC_ARM")"
    end
    on_intel do
      url "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$PREFIX-$VERSION-$MAC_X64"
      sha256 "$(sha "$MAC_X64")"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$PREFIX-$VERSION-$LIN_ARM"
      sha256 "$(sha "$LIN_ARM")"
    end
    on_intel do
      url "https://github.com/$OWNER/$REPO/releases/download/$VERSION/$PREFIX-$VERSION-$LIN_X64"
      sha256 "$(sha "$LIN_X64")"
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
)

ASSET_MAP=$(jq -n --arg v "$VERSION" \
  --arg base "https://github.com/$OWNER/$REPO/releases/download/$VERSION" \
  --arg p "$PREFIX" \
  --arg sha_lx "$(sha "$LIN_X64")" \
  --arg sha_la "$(sha "$LIN_ARM")" \
  --arg sha_mx "$(sha "$MAC_X64")" \
  --arg sha_ma "$(sha "$MAC_ARM")" \
  --arg sha_wx "$(sha "$WIN_X64")" \
  --arg sha_wa "$(sha "$WIN_ARM")" \
  '{
    v: $v,
    "linux-x64":   { url: "\($base)/\($p)-\($v)-x86_64-unknown-linux-musl.tar.gz",   sha256: $sha_lx },
    "linux-arm64": { url: "\($base)/\($p)-\($v)-aarch64-unknown-linux-musl.tar.gz", sha256: $sha_la },
    "macos-x64":   { url: "\($base)/\($p)-\($v)-x86_64-apple-darwin.tar.gz",        sha256: $sha_mx },
    "macos-arm64": { url: "\($base)/\($p)-\($v)-aarch64-apple-darwin.tar.gz",       sha256: $sha_ma },
    "win-x64":     { url: "\($base)/\($p)-\($v)-x86_64-pc-windows-msvc.zip",        sha256: $sha_wx },
    "win-arm64":   { url: "\($base)/\($p)-\($v)-aarch64-pc-windows-msvc.zip",       sha256: $sha_wa }
  }')

# Write outputs (or stdout if no path given)
if [[ -n "$BREW_OUT" ]]; then
  mkdir -p "$(dirname "$BREW_OUT")"
  printf "%s\n" "$BREW_FORMULA" > "$BREW_OUT"
  echo "[inject] wrote $BREW_OUT"
else
  printf "%s\n" "$BREW_FORMULA"
fi

if [[ -n "$ASSET_MAP_OUT" ]]; then
  mkdir -p "$(dirname "$ASSET_MAP_OUT")"
  printf "%s\n" "$ASSET_MAP" > "$ASSET_MAP_OUT"
  echo "[inject] wrote $ASSET_MAP_OUT"
else
  printf "%s\n" "$ASSET_MAP"
fi
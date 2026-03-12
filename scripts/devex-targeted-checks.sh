#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/devex-targeted-checks.sh [--base <git-ref>] [--mode <clippy|test|all>]

Runs fast, targeted checks only for crates changed since <git-ref>.

Options:
  --base <git-ref>   Base reference for diff (default: origin/master, fallback: HEAD~1)
  --mode <mode>      Check mode: clippy, test, or all (default: all)
  -h, --help         Show this help
USAGE
}

BASE_REF="origin/master"
MODE="all"

while (($# > 0)); do
  case "$1" in
    --base)
      BASE_REF="${2:-}"
      shift 2
      ;;
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$BASE_REF" ]]; then
  echo "--base requires a non-empty git reference" >&2
  exit 2
fi

if [[ "$MODE" != "clippy" && "$MODE" != "test" && "$MODE" != "all" ]]; then
  echo "--mode must be one of: clippy, test, all" >&2
  exit 2
fi

if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "⚠️  Base ref '$BASE_REF' not found; falling back to HEAD~1"
  BASE_REF="HEAD~1"
fi

if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  echo "❌ Could not resolve a valid base ref (tried origin/master and HEAD~1)" >&2
  exit 1
fi

mapfile -t crate_dirs < <(
  git diff --name-only "$BASE_REF"...HEAD \
    | rg '^crates/[^/]+/' -o \
    | cut -d/ -f1-2 \
    | sort -u
)

if (( ${#crate_dirs[@]} == 0 )); then
  echo "✅ No crate changes detected since $BASE_REF; skipping targeted checks"
  exit 0
fi

packages=()
for dir in "${crate_dirs[@]}"; do
  manifest="$dir/Cargo.toml"
  if [[ ! -f "$manifest" ]]; then
    continue
  fi

  pkg_name=$(awk -F' *= *' '
    /^\[package\]$/ { in_package=1; next }
    /^\[/ && $0 !~ /^\[package\]$/ { in_package=0 }
    in_package && /^name *=/ {
      gsub(/"/, "", $2)
      print $2
      exit
    }
  ' "$manifest")

  if [[ -n "$pkg_name" ]]; then
    packages+=("$pkg_name")
  fi
done

if (( ${#packages[@]} == 0 )); then
  echo "⚠️  Changed crates found, but no package names could be resolved"
  exit 1
fi

mapfile -t uniq_packages < <(printf '%s\n' "${packages[@]}" | sort -u)

package_args=()
for pkg in "${uniq_packages[@]}"; do
  package_args+=("-p" "$pkg")
done

echo "Detected changed packages since $BASE_REF:"
printf '  - %s\n' "${uniq_packages[@]}"

if [[ "$MODE" == "clippy" || "$MODE" == "all" ]]; then
  echo
  echo "▶ Running clippy for changed packages"
  cargo clippy "${package_args[@]}" --locked -- -D warnings -A missing_docs
fi

if [[ "$MODE" == "test" || "$MODE" == "all" ]]; then
  echo
  echo "▶ Running tests for changed packages"
  cargo test "${package_args[@]}" --lib --locked
fi

echo
echo "✅ Targeted checks completed"

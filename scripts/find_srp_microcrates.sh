#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Focused families that intentionally model single-responsibility microcrates.
readonly srp_prefixes=(
  "perl-module-"
  "perl-lsp-feature-"
  "perl-dap-"
  "perl-ts-"
  "perl-workspace-"
)

mapfile -t crate_dirs < <(find crates -mindepth 1 -maxdepth 1 -type d | sort)

srp_crates=()
other_crates=()

for dir in "${crate_dirs[@]}"; do
  crate_name="$(basename "$dir")"

  is_srp=0
  for prefix in "${srp_prefixes[@]}"; do
    if [[ "$crate_name" == "$prefix"* ]]; then
      is_srp=1
      break
    fi
  done

  # Some SRP crates do not use family prefixes; discover by crate metadata.
  if [[ $is_srp -eq 0 ]] && [[ -f "$dir/Cargo.toml" ]]; then
    if rg -q --max-count 1 '(?i)single responsibility|srp|microcrate' "$dir/Cargo.toml"; then
      is_srp=1
    fi
  fi

  if [[ $is_srp -eq 0 ]] && [[ -f "$dir/README.md" ]]; then
    if rg -q --max-count 1 '(?i)single responsibility|srp|microcrate' "$dir/README.md"; then
      is_srp=1
    fi
  fi

  if [[ $is_srp -eq 1 ]]; then
    srp_crates+=("$crate_name")
  else
    other_crates+=("$crate_name")
  fi
done

printf 'SRP microcrates (%d):\n' "${#srp_crates[@]}"
printf '  - %s\n' "${srp_crates[@]}"
printf '\nNon-SRP/composition crates (%d):\n' "${#other_crates[@]}"
printf '  - %s\n' "${other_crates[@]}"

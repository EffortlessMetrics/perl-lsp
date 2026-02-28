#!/usr/bin/env bash
set -euo pipefail

# Identify workspace crates that explicitly declare SRP/microcrate intent.
# A crate qualifies if any of these files mention "single responsibility" or "microcrate":
#   - README.md
#   - src/lib.rs
#   - src/main.rs

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

patterns=('single responsibility' 'microcrate')

printf "%-40s | %s\n" "crate" "matched file"
printf '%s\n' "$(printf '%.0s-' {1..72})"

for crate_dir in crates/*; do
    [[ -d "$crate_dir" ]] || continue

    crate_name="$(basename "$crate_dir")"
    matched_file=""

    for candidate in "README.md" "src/lib.rs" "src/main.rs"; do
        path="$crate_dir/$candidate"
        [[ -f "$path" ]] || continue

        for pattern in "${patterns[@]}"; do
            if rg -q -i "$pattern" "$path"; then
                matched_file="$candidate"
                break 2
            fi
        done
    done

    if [[ -n "$matched_file" ]]; then
        printf "%-40s | %s\n" "$crate_name" "$matched_file"
    fi
done | sort

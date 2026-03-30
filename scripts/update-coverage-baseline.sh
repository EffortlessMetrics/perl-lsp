#!/usr/bin/env bash
set -euo pipefail

lcov_path="${1:-lcov.info}"
baseline_path="${2:-.ci/coverage-baseline.txt}"

if [[ ! -f "$lcov_path" ]]; then
  echo "error: coverage file not found: $lcov_path" >&2
  exit 1
fi

baseline_dir="$(dirname "$baseline_path")"
mkdir -p "$baseline_dir"

get_existing_value() {
  local key="$1"

  if [[ ! -f "$baseline_path" ]]; then
    return 0
  fi

  grep -E "^${key}=" "$baseline_path" | head -n1 | cut -d= -f2- | sed 's/#.*$//' | tr -d '[:space:]'
}

coverage_stats="$(
  awk '
    /^BRF:/ { branch_found += substr($0, 5) + 0 }
    /^BRH:/ { branch_hit += substr($0, 5) + 0 }
    /^LF:/ { line_found += substr($0, 4) + 0 }
    /^LH:/ { line_hit += substr($0, 4) + 0 }
    END {
      branch_pct = branch_found > 0 ? (branch_hit / branch_found * 100.0) : 100.0
      line_pct = line_found > 0 ? (line_hit / line_found * 100.0) : 100.0
      printf "%.2f %.2f\n", branch_pct, line_pct
    }
  ' "$lcov_path"
)"

read -r current_branch_coverage current_line_coverage <<<"$coverage_stats"

coverage_scope="$(get_existing_value coverage_scope)"
allowed_drop_percentage="$(get_existing_value allowed_drop_percentage)"
target_branch_coverage="$(get_existing_value target_branch_coverage)"

coverage_scope="${coverage_scope:-perl-parser-lib}"
allowed_drop_percentage="${allowed_drop_percentage:-1.00}"
target_branch_coverage="${target_branch_coverage:-80.00}"

cat >"$baseline_path" <<EOF
# Branch coverage baseline policy for the parser coverage lane.
#
# Refresh with: just coverage-baseline-refresh
# Generated from: ${lcov_path}
schema_version=1
coverage_scope=${coverage_scope}
baseline_branch_coverage=${current_branch_coverage}
baseline_line_coverage=${current_line_coverage}
allowed_drop_percentage=${allowed_drop_percentage}
target_branch_coverage=${target_branch_coverage}
EOF

printf 'Updated %s\n' "$baseline_path"
printf '  Branch coverage baseline: %s%%\n' "$current_branch_coverage"
printf '  Line coverage baseline:   %s%%\n' "$current_line_coverage"


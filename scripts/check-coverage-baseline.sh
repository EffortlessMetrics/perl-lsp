#!/usr/bin/env bash
set -euo pipefail

lcov_path="${1:-lcov.info}"
baseline_path="${2:-.ci/coverage-baseline.txt}"
summary_path="${3:-${GITHUB_STEP_SUMMARY:-}}"

if [[ ! -f "$lcov_path" ]]; then
  echo "error: coverage file not found: $lcov_path" >&2
  exit 1
fi

if [[ ! -f "$baseline_path" ]]; then
  echo "error: baseline file not found: $baseline_path" >&2
  exit 1
fi

get_value() {
  local key="$1"
  local value

  value="$(grep -E "^${key}=" "$baseline_path" | head -n1 | cut -d= -f2- || true)"
  value="${value%%#*}"
  value="${value//[[:space:]]/}"
  printf '%s' "$value"
}

baseline_branch_coverage="$(get_value baseline_branch_coverage)"
baseline_line_coverage="$(get_value baseline_line_coverage)"
allowed_drop_percentage="$(get_value allowed_drop_percentage)"
target_branch_coverage="$(get_value target_branch_coverage)"

if [[ -z "$baseline_branch_coverage" || -z "$allowed_drop_percentage" || -z "$target_branch_coverage" ]]; then
  echo "error: baseline file is missing required keys" >&2
  exit 1
fi

coverage_stats="$(
  awk '
    /^BRF:/ { branch_found += substr($0, 5) + 0 }
    /^BRH:/ { branch_hit += substr($0, 5) + 0 }
    /^LF:/ { line_found += substr($0, 4) + 0 }
    /^LH:/ { line_hit += substr($0, 4) + 0 }
    END {
      branch_pct = branch_found > 0 ? (branch_hit / branch_found * 100.0) : 100.0
      line_pct = line_found > 0 ? (line_hit / line_found * 100.0) : 100.0
      printf "%.2f %.2f %d %d %d %d\n", branch_pct, line_pct, branch_hit, branch_found, line_hit, line_found
    }
  ' "$lcov_path"
)"

read -r current_branch_coverage current_line_coverage branch_hit branch_found line_hit line_found <<<"$coverage_stats"

drop_vs_baseline="$(awk -v baseline_branch="$baseline_branch_coverage" -v current_branch="$current_branch_coverage" 'BEGIN { printf "%.2f", baseline_branch - current_branch }')"
minimum_passing_branch_coverage="$(awk -v baseline_branch="$baseline_branch_coverage" -v allowed_drop="$allowed_drop_percentage" 'BEGIN { printf "%.2f", baseline_branch - allowed_drop }')"

gate_status="pass"
if awk -v drop="$drop_vs_baseline" -v allowed_drop="$allowed_drop_percentage" 'BEGIN { exit !(drop > allowed_drop) }'; then
  gate_status="fail"
fi

printf 'Branch coverage: %.2f%% (%s/%s)\n' "$current_branch_coverage" "$branch_hit" "$branch_found"
printf 'Line coverage:   %.2f%% (%s/%s)\n' "$current_line_coverage" "$line_hit" "$line_found"
printf 'Baseline branch: %.2f%%\n' "$baseline_branch_coverage"
if [[ -n "$baseline_line_coverage" ]]; then
  printf 'Baseline line:   %.2f%%\n' "$baseline_line_coverage"
fi
printf 'Allowed drop:    %.2f%%\n' "$allowed_drop_percentage"
printf 'Target branch:   %.2f%%\n' "$target_branch_coverage"
printf 'Gate threshold:  %.2f%%\n' "$minimum_passing_branch_coverage"

if [[ -n "$summary_path" ]]; then
  {
    printf '## Branch coverage gate\n\n'
    printf '| Metric | Value |\n'
    printf '| --- | ---: |\n'
    printf '| Gate status | %s |\n' "$gate_status"
    printf '| Branch coverage | %.2f%% (%s/%s) |\n' "$current_branch_coverage" "$branch_hit" "$branch_found"
    printf '| Line coverage | %.2f%% (%s/%s) |\n' "$current_line_coverage" "$line_hit" "$line_found"
    printf '| Baseline branch coverage | %.2f%% |\n' "$baseline_branch_coverage"
    if [[ -n "$baseline_line_coverage" ]]; then
      printf '| Baseline line coverage | %.2f%% |\n' "$baseline_line_coverage"
    fi
    printf '| Allowed drop | %.2f%% |\n' "$allowed_drop_percentage"
    printf '| Minimum passing branch coverage | %.2f%% |\n' "$minimum_passing_branch_coverage"
    printf '| Long-term target branch coverage | %.2f%% |\n' "$target_branch_coverage"
    printf '| Delta vs baseline | %.2f%% |\n' "$drop_vs_baseline"

    printf '\n'
    if [[ "$gate_status" == "pass" ]]; then
      printf '✅ Branch coverage is within the allowed regression budget.\n'
    else
      printf '❌ Branch coverage exceeded the allowed regression budget.\n'
    fi

    if awk -v current_branch="$current_branch_coverage" -v target_branch="$target_branch_coverage" 'BEGIN { exit !(current_branch < target_branch) }'; then
      printf '\n⚠️ Branch coverage is below the long-term target.\n'
    fi
  } >>"$summary_path"
fi

awk -v current_branch="$current_branch_coverage" \
    -v baseline_branch="$baseline_branch_coverage" \
    -v allowed_drop="$allowed_drop_percentage" \
    -v target_branch="$target_branch_coverage" '
  BEGIN {
    exit_code = 0
    drop = baseline_branch - current_branch

    if (drop > allowed_drop) {
      printf "error: branch coverage dropped by %.2f%%, which exceeds the allowed %.2f%%\n", drop, allowed_drop > "/dev/stderr"
      exit_code = 1
    }

    if (current_branch < target_branch) {
      printf "warning: branch coverage is below the long-term target of %.2f%%\n", target_branch > "/dev/stderr"
    }

    exit exit_code
  }
'

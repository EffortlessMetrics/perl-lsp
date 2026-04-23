#!/usr/bin/env bash
# Run merge gates and emit a receipt JSON under target/receipts.
# Usage: RUN_FULL=1 ./scripts/run-gates.sh   # optional full gate

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECEIPTS_DIR="$ROOT/target/receipts"
LOG_DIR="$RECEIPTS_DIR/logs"
ARTIFACT_DIR="$RECEIPTS_DIR/artifacts"

mkdir -p "$LOG_DIR" "$ARTIFACT_DIR"

policy_path="ci/gate-policy.yaml"

stamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
commit_sha="$(cd "$ROOT" && git rev-parse HEAD 2>/dev/null || echo UNVERIFIED)"
rustc_ver="$(rustc --version 2>/dev/null || echo UNVERIFIED)"
cargo_ver="$(cargo --version 2>/dev/null || echo UNVERIFIED)"
host_info="$(uname -a 2>/dev/null || echo UNVERIFIED)"

json_escape() {
  local s="$1"
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//$'\n'/\\n}
  printf '%s' "$s"
}

overall_failure=0

declare -a gate_ids gate_commands gate_required gate_status gate_exit_codes gate_durations gate_logs
declare -a gate_reasons

run_gate() {
  local gate_id="$1"
  local gate_cmd="$2"
  local required="$3"
  local reason="${4:-required merge gate}"
  local log_path="$LOG_DIR/${gate_id}.log"

  echo "==> Running ${gate_id}: ${gate_cmd}"
  local start end duration exit_code status
  start="$(date +%s)"

  set +e
  (cd "$ROOT" && bash -lc "$gate_cmd") 2>&1 | tee "$log_path"
  exit_code=${PIPESTATUS[0]}
  set -e

  end="$(date +%s)"
  duration=$((end - start))

  status="success"
  if [[ "$exit_code" -ne 0 ]]; then
    status="failure"
  fi

  gate_ids+=("$gate_id")
  gate_commands+=("$gate_cmd")
  gate_required+=("$required")
  gate_status+=("$status")
  gate_exit_codes+=("$exit_code")
  gate_durations+=("$duration")
  gate_logs+=("$log_path")
  gate_reasons+=("$reason")

  if [[ "$required" == "true" && "$exit_code" -ne 0 ]]; then
    overall_failure=1
  fi
}

run_gate "ci-gate" "just ci-gate" "true" "Always required for merge-readiness validation"
if [[ "${RUN_FULL:-}" == "1" ]]; then
  run_gate "ci-full" "just ci-full" "false" "RUN_FULL=1 requested expanded diagnostics"
fi

artifact_paths=()
for f in test-output.txt test-summary.json rustdoc.log doc-summary.json state.json; do
  if [[ -f "$ROOT/artifacts/$f" ]]; then
    cp -f "$ROOT/artifacts/$f" "$ARTIFACT_DIR/$f"
    artifact_paths+=("artifacts/$f")
  fi
done

# Collect debt status for receipt
debt_status=""
if command -v python3 >/dev/null 2>&1 && [[ -f "$ROOT/scripts/debt-report.py" ]]; then
  debt_status=$(python3 "$ROOT/scripts/debt-report.py" --json 2>/dev/null | python3 -c "
import sys, json
try:
    r = json.load(sys.stdin)
    s = r.get('summary', {})
    out = {
        'overall_status': s.get('overall_status', 'unknown'),
        'quarantined_tests': s.get('quarantined_tests', {}),
        'known_issues': s.get('known_issues', {}),
        'technical_debt': s.get('technical_debt', {}),
        'expired_quarantines': [x['name'] for x in r.get('details', {}).get('expired_quarantines', [])]
    }
    print(json.dumps(out))
except Exception:
    print('null')
" 2>/dev/null || echo "null")
fi

receipt_path="$RECEIPTS_DIR/receipt.json"
selected_scope="merge-gate"
if [[ "${RUN_FULL:-}" == "1" ]]; then
  selected_scope="merge-gate+full"
fi

failure_count=0
for i in "${!gate_ids[@]}"; do
  if [[ "${gate_status[$i]}" == "failure" ]]; then
    failure_count=$((failure_count + 1))
  fi
done

{
  echo "{"
  echo "  \"schema\": 1,"
  echo "  \"generated_at\": \"$(json_escape "$stamp")\","
  echo "  \"commit\": \"$(json_escape "$commit_sha")\","
  echo "  \"rustc\": \"$(json_escape "$rustc_ver")\","
  echo "  \"cargo\": \"$(json_escape "$cargo_ver")\","
  echo "  \"host\": \"$(json_escape "$host_info")\","
  echo "  \"scope\": \"$(json_escape "$selected_scope")\","
  echo "  \"selected_lanes\": ["
  for i in "${!gate_ids[@]}"; do
    printf "    \"%s\"" "$(json_escape "${gate_ids[$i]}")"
    if [[ $i -lt $(( ${#gate_ids[@]} - 1 )) ]]; then
      echo ","
    else
      echo ""
    fi
  done
  echo "  ],"
  echo "  \"reasons\": ["
  for i in "${!gate_ids[@]}"; do
    printf "    {\"lane\":\"%s\",\"reason\":\"%s\"}" \
      "$(json_escape "${gate_ids[$i]}")" \
      "$(json_escape "${gate_reasons[$i]}")"
    if [[ $i -lt $(( ${#gate_ids[@]} - 1 )) ]]; then
      echo ","
    else
      echo ""
    fi
  done
  echo "  ],"
  echo "  \"policy_path\": \"$(json_escape "$policy_path")\","
  echo "  \"gates\": ["

  for i in "${!gate_ids[@]}"; do
    gate_id="${gate_ids[$i]}"
    gate_cmd="${gate_commands[$i]}"
    gate_req="${gate_required[$i]}"
    gate_stat="${gate_status[$i]}"
    gate_code="${gate_exit_codes[$i]}"
    gate_dur="${gate_durations[$i]}"
    gate_log="${gate_logs[$i]}"

    printf "    {\"name\":\"%s\",\"command\":\"%s\",\"required\":%s,\"status\":\"%s\",\"exit_code\":%s,\"duration_seconds\":%s,\"log_path\":\"%s\"}" \
      "$(json_escape "$gate_id")" \
      "$(json_escape "$gate_cmd")" \
      "$gate_req" \
      "$(json_escape "$gate_stat")" \
      "$gate_code" \
      "$gate_dur" \
      "$(json_escape "$gate_log")"

    if [[ $i -lt $(( ${#gate_ids[@]} - 1 )) ]]; then
      echo ","
    else
      echo ""
    fi
  done

  echo "  ],"
  echo "  \"artifacts\": ["

  if [[ ${#artifact_paths[@]} -gt 0 ]]; then
    for i in "${!artifact_paths[@]}"; do
      printf "    \"%s\"" "$(json_escape "${artifact_paths[$i]}")"
      if [[ $i -lt $(( ${#artifact_paths[@]} - 1 )) ]]; then
        echo ","
      else
        echo ""
      fi
    done
  fi

  echo "  ],"
  echo "  \"failures\": {"
  echo "    \"count\": $failure_count,"
  echo "    \"repro\": ["
  repro_written=0
  for i in "${!gate_ids[@]}"; do
    if [[ "${gate_status[$i]}" == "failure" ]]; then
      if [[ $repro_written -eq 1 ]]; then
        echo ","
      fi
      printf "      \"%s\"" "$(json_escape "${gate_commands[$i]}")"
      repro_written=1
    fi
  done
  if [[ $repro_written -eq 1 ]]; then
    echo ""
  fi
  echo "    ]"
  echo "  },"
  echo "  \"baselines\": ["
  echo "    \".ci/common-corpus-manifest.txt\","
  echo "    \".ci/parser-corpus-baseline.json\","
  echo "    \".ci/public-api-baselines/\""
  echo "  ],"
  echo "  \"next_actions\": ["
  if [[ "$failure_count" -gt 0 ]]; then
    echo "    \"Re-run the failing command(s) listed in failures.repro\","
    echo "    \"Inspect matching logs under target/receipts/logs/\","
    echo "    \"Apply fixes, then re-run ./scripts/run-gates.sh\""
  else
    echo "    \"No action needed: all selected lanes passed\""
  fi
  echo "  ],"
  echo "  \"debt_status\": ${debt_status:-null}"
  echo "}"
} > "$receipt_path"

echo "Receipt written to: $receipt_path"

if [[ "$overall_failure" -ne 0 ]]; then
  echo "One or more required gates failed."
  exit 1
fi

#!/usr/bin/env bash
# Usage: bash scripts/swarm-summary.sh [ops-dir]
# Prints a readable summary of swarm-metrics.jsonl.
OPS_DIR="${1:-.ops-perl-lsp}"
METRICS="${OPS_DIR}/swarm-metrics.jsonl"

if [[ ! -f "${METRICS}" ]]; then
  echo "No metrics file found at ${METRICS}"
  exit 1
fi

echo "=== Swarm Metrics Summary ==="
echo "Total entries: $(wc -l < "${METRICS}")"
echo ""
echo "By event type:"
jq -r '.event // .action // "(none)"' "${METRICS}" | sort | uniq -c | sort -rn
echo ""
echo "By agent type:"
jq -r '.agent_type // .type // "(none)"' "${METRICS}" | sort | uniq -c | sort -rn
echo ""
echo "Recent completions (last 5):"
jq -r 'select(.event=="task_completed") | [.ts, .cwd] | @tsv' "${METRICS}" | tail -5
echo ""
echo "Recent stops (last 5):"
jq -r 'select(.event=="subagent_stop") | [.ts, .agent_type, .worktree_path] | @tsv' "${METRICS}" | tail -5

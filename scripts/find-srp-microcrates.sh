#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ ! -d crates ]]; then
  echo "error: crates/ directory not found" >&2
  exit 1
fi

collect() {
  local label="$1"
  shift
  local -a matches=()

  while IFS= read -r dir; do
    matches+=("${dir#crates/}")
  done < <(find crates -maxdepth 1 -mindepth 1 -type d "$@" | sort)

  echo "$label (${#matches[@]})"
  for crate in "${matches[@]}"; do
    echo "  - $crate"
  done
  echo
}

echo "SRP microcrate inventory"
echo "========================"
echo

collect "Module microcrates" -name 'perl-module-*'
collect "LSP feature governance microcrates" -name 'perl-lsp-feature-*'
collect "LSP support microcrates" \( -name 'perl-lsp-cancellation' -o -name 'perl-lsp-launcher' \)
collect "DAP microcrates" \( -name 'perl-dap-breakpoint' -o -name 'perl-dap-eval' -o -name 'perl-dap-stack' -o -name 'perl-dap-variables' \)
collect "Workspace/index microcrates" \( -name 'perl-workspace-discovery' -o -name 'perl-workspace-index-slo' \)
collect "Cross-cutting utility microcrates" \( -name 'perl-content-length-framing' -o -name 'perl-path-security' -o -name 'perl-position-tracking' -o -name 'perl-qualified-name' -o -name 'perl-source-file' -o -name 'perl-text-line' -o -name 'perl-uri' \)

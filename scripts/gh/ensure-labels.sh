#!/usr/bin/env bash
set -euo pipefail

# Ensure gh auth works
gh auth status >/dev/null

# Get existing labels as a searchable string
existing="$(gh label list --limit 1000 --json name --jq '.[].name' | tr '\n' '|')"
existing="|${existing}"  # Prefix with | for boundary matching

ensure() {
  local name="$1"
  local color="$2"
  local desc="$3"

  if [[ "$existing" == *"|$name|"* ]]; then
    echo "✓ label exists: $name"
  else
    echo "→ creating label: $name"
    gh label create "$name" --color "$color" --description "$desc"
  fi
}

echo "=== Type Labels ==="
ensure "type:bug"            "d73a4a" "Something is incorrect or broken"
ensure "type:enhancement"    "a2eeef" "New capability or improvement"
ensure "type:chore"          "cfd3d7" "Maintenance and cleanup"
ensure "type:infrastructure" "0052cc" "CI/build/release/ops work"
ensure "type:docs"           "0075ca" "Documentation changes"

echo ""
echo "=== Priority Labels ==="
# Note: You already have priority:critical, priority:high, etc.
# Adding P0-P3 as aliases for faster typing
ensure "P0-critical" "b60205" "Blocker / must fix immediately"
ensure "P1-high"     "d93f0b" "High impact, fix this sprint"
ensure "P2-medium"   "fbca04" "Normal priority"
ensure "P3-low"      "0e8a16" "Nice to have / backlog"

echo ""
echo "=== Status Labels ==="
# You already have: blocked, in-progress
ensure "status:blocked"      "5319e7" "Blocked by external dependency"
ensure "status:ready"        "0e8a16" "Ready to start"
ensure "status:in-progress"  "1d76db" "Actively being worked"
ensure "status:needs-triage" "ededed" "Needs review / categorization"

echo ""
echo "=== Area Labels ==="
# You already have unprefixed: parser, lsp, tests, infrastructure
# Adding prefixed versions for consistency
ensure "area:ci"      "0052cc" "CI and automation"
ensure "area:parser"  "f9d0c4" "Perl parser"
ensure "area:lsp"     "c5def5" "Language Server Protocol"
ensure "area:dap"     "bfdadc" "Debug Adapter Protocol"
ensure "area:tests"   "e4e669" "Testing infrastructure"
ensure "area:docs"    "0075ca" "Documentation"
ensure "area:lexer"   "d4edda" "Lexer and tokenization"
ensure "area:semantic" "c2e0c6" "Semantic analysis"

echo ""
echo "=== Done ==="
echo "Label taxonomy is ready. Your automated workflows (gate:*, review:*, fix:*) remain intact."

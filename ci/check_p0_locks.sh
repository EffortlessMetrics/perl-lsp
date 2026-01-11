#!/bin/bash
# CI ratchet gate: enforce zero lock().unwrap() in LSP server_impl (P0 concurrency safety)
set -euo pipefail

# Target directory for P0 lock safety enforcement
target_dir="crates/perl-parser/src/lsp/server_impl"

if [ ! -d "$target_dir" ]; then
  echo "⚠️  Directory not found: $target_dir"
  echo "Skipping P0 lock check (directory may have been restructured)"
  exit 0
fi

# Search for dangerous lock patterns
# Patterns: lock().unwrap(), read().unwrap(), write().unwrap()
echo "Checking for unsafe lock patterns in $target_dir..."
echo "Target: 0 occurrences (P0 concurrency safety requirement)"
echo ""

if command -v rg &>/dev/null; then
  # Use ripgrep for better output
  matches=$(rg 'lock\(\)\.unwrap\(\)|read\(\)\.unwrap\(\)|write\(\)\.unwrap\(\)' "$target_dir" -n 2>/dev/null || true)
else
  # Fallback to grep
  matches=$(grep -rn 'lock()\.unwrap()\|read()\.unwrap()\|write()\.unwrap()' "$target_dir" 2>/dev/null || true)
fi

if [ -z "$matches" ]; then
  echo "✅ PASS: No unsafe lock patterns found"
  echo "   All lock operations use proper error handling"
  exit 0
else
  # Count occurrences
  count=$(echo "$matches" | grep -c . || echo 0)

  echo "❌ FAIL: Found $count unsafe lock pattern(s)"
  echo ""
  echo "Locations:"
  echo "$matches"
  echo ""
  echo "ERROR: P0 concurrency safety violation"
  echo ""
  echo "lock().unwrap(), read().unwrap(), and write().unwrap() can panic and crash the LSP server."
  echo ""
  echo "Required actions:"
  echo "  1. Replace .unwrap() with proper error handling"
  echo "  2. Use .map_err() to convert PoisonError to LspError"
  echo "  3. Return errors to caller instead of panicking"
  echo ""
  echo "Example replacement:"
  echo "  // Before (unsafe):"
  echo "  let docs = self.documents.lock().unwrap();"
  echo ""
  echo "  // After (safe):"
  echo "  let docs = self.documents.lock()"
  echo "      .map_err(|e| LspError::internal_error(format!(\"Lock poisoned: {}\", e)))?;"
  echo ""
  echo "See docs/LSP_IMPLEMENTATION_GUIDE.md for concurrency patterns"
  exit 1
fi

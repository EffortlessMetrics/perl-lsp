#!/usr/bin/env bash
# Quick check that mirrors exactly what CI runs
set -euo pipefail

echo "=== Quick CI Mirror Check ==="
echo ""

echo "1. Format check"
cargo fmt --all -- --check

echo ""
echo "2. Clippy (strict on first-party)"
cargo clippy -p perl-parser --all-targets --all-features -- -D warnings
cargo clippy -p perl-lexer --all-targets --all-features -- -D warnings

echo ""
echo "3. Clippy (smoke check on rest)"
cargo clippy --workspace --all-targets --all-features \
  --exclude perl-parser --exclude perl-lexer || true

echo ""  
echo "4. Docs (strict)"
RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls" \
  cargo doc --workspace --no-deps

echo ""
echo "5. Tests (workspace, lib+bins+tests, no examples)"
cargo test --workspace --all-features

echo ""
echo "6. Ignored baseline"
./ci/check_ignored.sh

echo ""
echo "7. Cargo deny (if available)"
if command -v cargo-deny &> /dev/null; then
    cargo deny check
else
    echo "cargo-deny not installed (skipping)"
fi

echo ""
echo "✅ All checks complete"
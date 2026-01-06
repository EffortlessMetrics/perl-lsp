#!/usr/bin/env bash
# Install git hooks for perl-lsp development
# Usage: bash scripts/install-githooks.sh
set -euo pipefail

mkdir -p .git/hooks

cat > .git/hooks/pre-push <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

echo "🚪 Running local gate before push: nix develop -c just ci-gate"
echo "   (Skip with: git push --no-verify)"
echo ""

# Try nix develop first, fall back to just alone
if command -v nix &>/dev/null && [ -f flake.nix ]; then
    nix develop -c just ci-gate
elif command -v just &>/dev/null; then
    just ci-gate
else
    echo "⚠️  Neither 'nix develop' nor 'just' available, skipping pre-push gate"
    echo "   Install just: cargo install just"
    exit 0
fi
EOF

chmod +x .git/hooks/pre-push
echo "✅ Installed pre-push hook"
echo "   The hook runs 'nix develop -c just ci-gate' before each push"
echo "   Skip with: git push --no-verify"

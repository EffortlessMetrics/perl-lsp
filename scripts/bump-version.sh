#!/bin/bash
set -e

# Version bump script for Perl Language Server

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.6.1"
    exit 1
fi

NEW_VERSION="$1"
echo "🔄 Bumping version to $NEW_VERSION"

# Files to update
CARGO_FILES=(
    "crates/perl-lexer/Cargo.toml"
    "crates/perl-parser/Cargo.toml"
)

PACKAGE_JSON="vscode-extension/package.json"

# Update Cargo.toml files
for file in "${CARGO_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "  Updating $file..."
        sed -i "s/^version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" "$file"
    fi
done

# Update package.json
if [ -f "$PACKAGE_JSON" ]; then
    echo "  Updating $PACKAGE_JSON..."
    # Use a more robust approach for package.json
    node -e "
        const fs = require('fs');
        const pkg = JSON.parse(fs.readFileSync('$PACKAGE_JSON', 'utf8'));
        pkg.version = '$NEW_VERSION';
        fs.writeFileSync('$PACKAGE_JSON', JSON.stringify(pkg, null, 2) + '\\n');
    "
fi

# Update debug adapter version strings
echo "  Updating version strings in source files..."
find crates/perl-parser/src -name "*.rs" -type f -exec sed -i "s/Perl Language Server v[0-9]\+\.[0-9]\+\.[0-9]\+/Perl Language Server v$NEW_VERSION/g" {} \;
find crates/perl-parser/src -name "*.rs" -type f -exec sed -i "s/Perl Debug Adapter v[0-9]\+\.[0-9]\+\.[0-9]\+/Perl Debug Adapter v$NEW_VERSION/g" {} \;

# Update README.md version references
echo "  Updating README.md..."
sed -i "s/perl-parser = \"[0-9]\+\.[0-9]\+\"/perl-parser = \"${NEW_VERSION%.*}\"/" README.md
sed -i "s/perl-language-server-[0-9]\+\.[0-9]\+\.[0-9]\+\.vsix/perl-language-server-$NEW_VERSION.vsix/g" README.md

# Show changes
echo ""
echo "✅ Version bumped to $NEW_VERSION"
echo ""
echo "📋 Changed files:"
git status --short

echo ""
echo "💡 Next steps:"
echo "1. Review the changes: git diff"
echo "2. Commit: git commit -am 'chore: bump version to $NEW_VERSION'"
echo "3. Run release script: ./scripts/release.sh"
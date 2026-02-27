# Publishing the Perl Language Server Extension

## Prerequisites

1. **Node.js and npm** installed
2. **Visual Studio Code** installed
3. **vsce** (Visual Studio Code Extension manager) installed:
   ```bash
   npm install -g @vscode/vsce
   ```
4. **Publisher account** on Visual Studio Marketplace

## Build Process

### 1. Build the LSP Binary

First, ensure the perl-lsp binary is built:

```bash
# From the project root
cd ..
cargo build -p perl-parser --bin perl-lsp --release
```

### 2. Build the Extension

```bash
# From project root
cargo xtask release <version>
```

Or manually:

```bash
# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Bundle LSP binary
npm run bundle-lsp

# Package extension
npm run package
```

### 3. Test Locally

Install and test the extension locally:

```bash
# Install the VSIX file
code --install-extension perl-language-server-*.vsix

# Open test file
code test/sample.pl
```

Test these features:
- [ ] Syntax highlighting works
- [ ] Diagnostics appear for syntax errors
- [ ] Format document (Shift+Alt+F) works (if perltidy installed)
- [ ] Go to definition (F12) works
- [ ] Hover shows information
- [ ] Auto-completion triggers

### 4. Cross-Platform Binaries

For marketplace release, build for all platforms:

```bash
# Linux x64
cargo build --target x86_64-unknown-linux-gnu --release

# macOS x64
cargo build --target x86_64-apple-darwin --release

# macOS ARM64
cargo build --target aarch64-apple-darwin --release

# Windows x64
cargo build --target x86_64-pc-windows-msvc --release
```

Place binaries in appropriate directories:
- `bin/linux-x64/perl-lsp`
- `bin/darwin-x64/perl-lsp`
- `bin/darwin-arm64/perl-lsp`
- `bin/win32-x64/perl-lsp.exe`

### 5. Create Publisher

If you haven't already:

1. Go to https://marketplace.visualstudio.com/manage
2. Create a publisher ID (e.g., "tree-sitter-perl")
3. Get a Personal Access Token from Azure DevOps

### 6. Login to vsce

```bash
vsce login <publisher-id>
# Enter your Personal Access Token when prompted
```

### 7. Publish

```bash
# Publish to marketplace
npm run publish

# Or with version bump
vsce publish minor  # 0.5.0 -> 0.6.0
vsce publish major  # 0.5.0 -> 0.9.x
vsce publish 0.5.1  # Specific version
```

## Post-Publishing

1. **Verify on Marketplace**
   - Go to https://marketplace.visualstudio.com/
   - Search for "Perl Language Server"
   - Verify description, screenshots, etc.

2. **Update Documentation**
   - Update main README.md with marketplace link
   - Add installation instructions
   - Update CHANGELOG.md

3. **Create GitHub Release**
   - Tag the release: `git tag vscode-extension-v0.5.0`
   - Create release on GitHub
   - Attach the .vsix file

## Maintenance

### Updating the Extension

1. Update version in `package.json`
2. Update `CHANGELOG.md`
3. Rebuild and test
4. Publish update: `vsce publish`

### Monitoring

- Check reviews and ratings on marketplace
- Monitor GitHub issues
- Respond to user feedback

## Troubleshooting

### "Missing publisher name"
Update `package.json` with your publisher ID.

### "Personal Access Token expired"
Create a new token and login again with `vsce login`.

### Binary not found
Ensure `bundle-lsp.js` correctly detects platform and copies binaries.

### Large package size
Check `.vscodeignore` is excluding unnecessary files.
# Publishing the Perl Language Server Extension

## Prerequisites

> **⚠️ Deprecation Notice:** The Visual Studio Marketplace listing is deprecated. Please use the [Open VSX Registry](https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs) for publishing.

1. **Node.js and npm** installed
2. **Visual Studio Code** installed
3. **vsce** (Visual Studio Code Extension manager) and **ovsx** (Open VSX CLI) installed:
   ```bash
   npm install -g @vscode/vsce ovsx
   ```
4. **Publisher account** on Visual Studio Marketplace (deprecated)
5. **Open VSX access token** for `EffortlessMetrics` publisher

## Build Process

### 1. Build the LSP Binary

First, ensure the `perllsp` binary is built:

```bash
# From the project root
cd ..
cargo build -p perllsp --release
```

### 2. Build and Validate the Extension

```bash
# From vscode-extension/
npm install
npm run verify:marketplace
```

`verify:marketplace` runs TypeScript compilation, bundles the local platform binary, and generates the Marketplace `.vsix` package.

To validate the Open VSX tooling before publishing:

```bash
npm run check:openvsx
```

Open VSX publishes the same `perl-lsp-rs-<version>.vsix` artifact used for the Visual Studio Marketplace, so the main extra preflight is confirming the `ovsx` CLI is installed and authenticated.

### 3. Test Locally

Install and test the extension locally:

```bash
# Install the VSIX file
code --install-extension perl-lsp-rs-*.vsix

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
- `bin/linux-x64/perllsp`
- `bin/darwin-x64/perllsp`
- `bin/darwin-arm64/perllsp`
- `bin/win32-x64/perllsp.exe`

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
# Publish to Visual Studio Marketplace
npm run publish -- --pat "$VSCE_PAT"

# Publish the Open VSX-specific package
npm run publish:openvsx -- perl-lsp-rs-*.vsix --pat "$OVSX_PAT"

# Or publish with version bump on Marketplace
vsce publish minor  # 0.5.0 -> 0.6.0
vsce publish major  # 0.5.0 -> 0.9.x
vsce publish 0.5.1  # Specific version
```

## Post-Publishing

1. **Verify on Marketplace and Open VSX**
   - Go to https://marketplace.visualstudio.com/
   - Search for "Perl Language Server"
   - Verify description, screenshots, install command, and version
   - Open https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs
   - Verify the Open VSX listing, version, and VSCodium install instructions

2. **Update Documentation**
   - Update main README.md with marketplace link
   - Refresh the manually maintained VS Marketplace installs badge count/date in:
     - `/README.md`
     - `/vscode-extension/README.md`
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
## Marketplace Launch Checklist

Before first public launch:

- [ ] Confirm `package.json` metadata is complete (`publisher`, `icon`, repository links, categories, keywords).
- [ ] Ensure `README.md` has clear install + configuration guidance.
- [ ] Ensure `CHANGELOG.md` includes release notes for the exact published version.
- [ ] Run `npm run verify:marketplace` and install the generated `.vsix` locally.
- [ ] Validate extension activation in a clean profile (`code --user-data-dir <tmpdir>`).
- [ ] Verify binary download fallback works when no bundled binary exists for the host platform.
- [ ] Publish as pre-release first (recommended for initial alpha), then promote to stable after validation feedback.

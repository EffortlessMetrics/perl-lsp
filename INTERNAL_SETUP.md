# Perl LSP v0.8.3-rc1 Internal Setup

## Quick Setup (3 Options)

### Option 1: Local Binary (Simplest)
```bash
# Extract and install the binary
tar -xzf releases/v0.8.3-rc1/perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz
sudo cp perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu/perl-lsp /usr/local/bin/
perl-lsp --version

# Configure VS Code
# Add to .vscode/settings.json:
{
  "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
  "perl-lsp.autoDownload": false
}
```

### Option 2: Internal Server Auto-Download
```json
// Host files on internal server with this structure:
// https://internal.example.com/perl-lsp/
//   ├── perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz
//   ├── perl-lsp-v0.8.3-rc1-x86_64-apple-darwin.tar.gz
//   └── SHA256SUMS

// Configure VS Code (.vscode/settings.json):
{
  "perl-lsp.downloadBaseUrl": "https://internal.example.com/perl-lsp",
  "perl-lsp.channel": "tag",
  "perl-lsp.versionTag": "v0.8.3-rc1"
}
```

### Option 3: Bundle with Extension
```bash
# Package extension with binary
cd vscode-extension
npm ci
npm run compile
npx vsce package

# Share perl-lsp-vscode-0.1.0.vsix with team
# Install: code --install-extension perl-lsp-vscode-0.1.0.vsix
```

## SHA256 Checksums
- Linux x86_64: `54461cc60adbad890deb1114bb8ec1fedd5ba1fd0854434fa79c02f97ba6414b`

## What's Fixed in v0.8.3-rc1
✅ VS Code extension activation events (no warnings)
✅ Internal distribution support via `downloadBaseUrl`
✅ Homebrew formula with SHA256 (Linux x86_64)
✅ Release workflow hardened for public launch

## Testing Commands
```bash
# Test LSP protocol
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio

# Test with file
perl-lsp --stdio < test.pl

# VS Code: Ctrl+Shift+P → "Perl LSP: Show Server Version"
```

## Public Launch Checklist
When ready to go public:
1. Enable GitHub Actions budget
2. Tag final: `git tag v0.8.3 && git push --tags`
3. Update Homebrew tap after assets upload
4. Publish to VS Code marketplace with VSCE_PAT
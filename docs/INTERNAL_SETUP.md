# Perl LSP v0.8.3-rc1 Internal Setup

## Quick Setup (3 Options)

### Option 1: Local Binary (Simplest)

```bash
# Extract and install the binary
tar -xzf releases/v0.8.3-rc1/perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz
sudo cp perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu/perl-lsp /usr/local/bin/
perl-lsp --version
```

VS Code settings:
```json
{
  "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
  "perl-lsp.autoDownload": false
}
```

### Option 2: Internal Server (Auto-download)

Host the files on an internal server:
- `perl-lsp-v0.8.3-rc1-<platform>.tar.gz`
- `SHA256SUMS` file

VS Code settings:
```json
{
  "perl-lsp.downloadBaseUrl": "https://internal.company.com/perl-lsp",
  "perl-lsp.channel": "tag",
  "perl-lsp.versionTag": "v0.8.3-rc1"
}
```

### Option 3: Bundle with Extension

Package the VSIX with binary included:
```bash
cd vscode-extension
npm ci
npm run compile
npx vsce package
```

Share `perl-lsp-vscode-0.1.0.vsix` with team.

## Testing

```bash
# Basic test
perl-lsp --version

# LSP test
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio
```

## Public Release Checklist

When ready to go public:

1. **Enable GitHub Actions**: Repo Settings → Actions → Enable
2. **Tag Release**: `git tag v0.8.3 && git push --tags`
3. **Update Homebrew**: `./scripts/update-homebrew.sh v0.8.3`
4. **VS Code Marketplace**: `npx vsce publish` (with VSCE_PAT)

## Current Status

✅ Internal RC ready for testing
✅ VS Code extension configured for internal use
✅ Local build script for additional platforms
✅ Release workflows ready for public repo

The system is fully prepared for both internal testing and eventual public release.
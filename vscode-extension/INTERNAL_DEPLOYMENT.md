# Internal Deployment Guide

This guide covers deploying the Perl LSP VS Code extension for internal use without GitHub Actions.

## Overview

The extension supports three deployment modes:

1. **Local Binary Path** - Point to a locally installed `perl-lsp` binary
2. **Internal File Server** - Host binaries on an internal web server  
3. **Bundled Extension** - Package the binary directly with the extension

## Option 1: Local Binary Path (Recommended)

The simplest approach for internal deployment.

### Setup

1. **Install perl-lsp binary** on each developer machine:
   ```bash
   # Build from source
   cargo build -p perl-parser --bin perl-lsp --release
   
   # Copy to standard location
   sudo cp target/release/perl-lsp /usr/local/bin/
   
   # Or install via Cargo
   cargo install --path crates/perl-parser --bin perl-lsp
   ```

2. **Configure VS Code** workspace settings (`.vscode/settings.json`):
   ```json
   {
     "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
     "perl-lsp.autoDownload": false
   }
   ```

3. **Package and distribute extension**:
   ```bash
   cd vscode-extension
   npm install
   npm run compile
   npx vsce package
   ```

4. **Install extension** on developer machines:
   ```bash
   code --install-extension perl-lsp-0.8.3.vsix
   ```

### Benefits
- Simple setup and maintenance
- No network dependencies after initial install
- Full control over binary versions
- Works offline

## Option 2: Internal File Server

Host binaries on an internal web server for automatic distribution.

### Setup

1. **Prepare binary releases** on your internal server:
   ```bash
   # Create directory structure
   mkdir -p /var/www/perl-lsp-binaries
   cd /var/www/perl-lsp-binaries
   
   # Copy your pre-built binaries
   # Naming convention: perl-lsp-VERSION-TARGET.tar.gz
   cp perl-lsp-v0.8.3-x86_64-unknown-linux-gnu.tar.gz .
   cp perl-lsp-v0.8.3-x86_64-apple-darwin.tar.gz .
   cp perl-lsp-v0.8.3-x86_64-pc-windows-msvc.zip .
   
   # Optional: Create checksum file
   sha256sum *.tar.gz *.zip > SHA256SUMS
   ```

2. **Configure web server** (nginx example):
   ```nginx
   server {
       listen 80;
       server_name internal-binaries.yourcompany.com;
       root /var/www/perl-lsp-binaries;
       
       location / {
           autoindex on;
           add_header Access-Control-Allow-Origin *;
       }
   }
   ```

3. **Configure VS Code** workspace settings:
   ```json
   {
     "perl-lsp.autoDownload": true,
     "perl-lsp.downloadBaseUrl": "https://internal-binaries.yourcompany.com",
     "perl-lsp.versionTag": "v0.8.3"
   }
   ```

### Benefits
- Automatic updates when new versions are uploaded
- Centralized binary management
- Platform-specific downloads
- Version control

## Option 3: Bundled Extension

Package the binary directly with the extension.

### Setup

1. **Build binaries** for target platforms:
   ```bash
   # Linux
   cargo build -p perl-parser --bin perl-lsp --release --target x86_64-unknown-linux-gnu
   
   # macOS  
   cargo build -p perl-parser --bin perl-lsp --release --target x86_64-apple-darwin
   
   # Windows
   cargo build -p perl-parser --bin perl-lsp --release --target x86_64-pc-windows-msvc
   ```

2. **Create binary directory structure**:
   ```bash
   cd vscode-extension
   mkdir -p bin/linux-x64 bin/darwin-x64 bin/win32-x64
   
   # Copy platform-specific binaries
   cp ../target/x86_64-unknown-linux-gnu/release/perl-lsp bin/linux-x64/
   cp ../target/x86_64-apple-darwin/release/perl-lsp bin/darwin-x64/
   cp ../target/x86_64-pc-windows-msvc/release/perl-lsp.exe bin/win32-x64/
   ```

3. **Package extension**:
   ```bash
   npm install
   npm run compile
   npx vsce package
   ```

4. **Configure workspace** (optional - binaries auto-detected):
   ```json
   {
     "perl-lsp.autoDownload": false
   }
   ```

### Benefits
- Zero external dependencies
- Guaranteed binary availability
- Works in air-gapped environments

### Drawbacks
- Large extension package size
- Must rebuild extension for updates
- Platform-specific builds required

## Team Configuration

Copy the provided `.vscode/settings.json` to your project root and customize:

```json
{
  "perl-lsp.serverPath": "/path/to/your/perl-lsp",
  "perl-lsp.autoDownload": false,
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "vendor/lib"
  ]
}
```

## Binary Building

### Prerequisites
- Rust toolchain (1.70+)
- Cargo

### Build Commands
```bash
# Quick build for testing
cargo build -p perl-parser --bin perl-lsp

# Optimized release build
cargo build -p perl-parser --bin perl-lsp --release

# Cross-platform builds (with targets installed)
cargo build -p perl-parser --bin perl-lsp --release --target x86_64-unknown-linux-gnu
cargo build -p perl-parser --bin perl-lsp --release --target x86_64-apple-darwin  
cargo build -p perl-parser --bin perl-lsp --release --target x86_64-pc-windows-msvc

# Install targets if needed
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add x86_64-pc-windows-msvc
```

### Verification
```bash
# Test binary works
./target/release/perl-lsp --version
./target/release/perl-lsp --help

# Quick LSP test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/release/perl-lsp --stdio
```

## Troubleshooting

### Binary Not Found
1. Check `perl-lsp.serverPath` setting
2. Verify binary permissions (`chmod +x`)
3. Test binary directly: `perl-lsp --version`

### Download Failures
1. Check `perl-lsp.downloadBaseUrl` setting
2. Verify internal server accessibility
3. Check file naming conventions
4. Review VS Code output panel logs

### Extension Issues
1. Check VS Code extension logs (Output > Perl Language Server)
2. Restart VS Code
3. Reload window (`Ctrl+Shift+P` > Developer: Reload Window)
4. Verify extension activation for `.pl` files

### Network Issues
1. Configure proxy settings in VS Code
2. Check firewall rules for internal servers
3. Use `serverPath` for offline environments

## Support

For internal deployment issues:
1. Check the VS Code Output panel (Perl Language Server)
2. Enable debug logging: `"perl-lsp.trace.server": "verbose"`
3. Test binary independently: `perl-lsp --stdio --log`
4. Verify workspace configuration matches your setup

This approach ensures the extension works both internally (with overrides) and publicly (default behavior) when released.
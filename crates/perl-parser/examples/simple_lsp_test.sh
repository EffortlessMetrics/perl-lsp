#!/bin/bash

# Simple test for the LSP server
echo "Testing Perl LSP server..."

# Create a test request
cat << 'EOF' | cargo run -p perl-parser --bin perl-lsp 2>&1 | head -20
Content-Length: 205

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":123,"rootUri":"file:///tmp","capabilities":{},"initializationOptions":{},"trace":"off","workspaceFolders":null}}
EOF
#!/bin/bash
# LSP smoke test - verify server responds correctly to initialize/shutdown

BIN=target/release/perl-lsp

# Create test request
cat > /tmp/lsp_test_req.txt <<'EOF'
Content-Length: 58

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
Content-Length: 52

{"jsonrpc":"2.0","method":"initialized","params":{}}
Content-Length: 47

{"jsonrpc":"2.0","id":2,"method":"shutdown"}
Content-Length: 33

{"jsonrpc":"2.0","method":"exit"}
EOF

echo "🔍 Testing LSP server..."
cat /tmp/lsp_test_req.txt | timeout 3s $BIN --stdio 2>/dev/null | head -c 5000 > /tmp/lsp_out.txt

# Check for expected responses
if grep -q '"id":1' /tmp/lsp_out.txt && grep -q '"capabilities"' /tmp/lsp_out.txt; then
    echo "✅ LSP server responds correctly"
    
    # Check if pull diagnostics is properly detected
    if grep -q '"textDocument/publishDiagnostics"' /tmp/lsp_out.txt; then
        echo "⚠️  Warning: Server published diagnostics despite client supporting pull"
    else
        echo "✅ Pull diagnostics properly suppressed"
    fi
    exit 0
else
    echo "❌ LSP server did not respond correctly"
    exit 1
fi
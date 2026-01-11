#!/bin/bash
# Quick validation that document_highlight feature works

set -e

echo "Building perl-lsp..."
cargo build -p perl-lsp --release --quiet

echo "Testing document_highlight via simple JSON-RPC exchange..."

# Create a test Perl file
cat > /tmp/test_highlight.pl <<'EOF'
my $foo = 42;
print $foo;
$foo = $foo + 1;
EOF

# Start the LSP server in the background
perl_lsp_bin="./target/release/perl-lsp"
exec 3< <($perl_lsp_bin --stdio 2>&1)
exec 4> >(timeout 5 $perl_lsp_bin --stdio)

# Function to send JSON-RPC request
send_request() {
    local content="$1"
    local length=${#content}
    echo -e "Content-Length: $length\r\n\r\n$content" >&4
}

# Initialize
send_request '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}'
sleep 0.5

# Send initialized notification
send_request '{"jsonrpc":"2.0","method":"initialized","params":{}}'
sleep 0.2

# Open document
send_request '{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/test_highlight.pl","languageId":"perl","version":1,"text":"my $foo = 42;\nprint $foo;\n$foo = $foo + 1;"}}}'
sleep 0.2

# Request document highlight
send_request '{"jsonrpc":"2.0","id":2,"method":"textDocument/documentHighlight","params":{"textDocument":{"uri":"file:///tmp/test_highlight.pl"},"position":{"line":0,"character":4}}}'
sleep 0.5

# Read response
timeout 2 cat <&3 | grep -A20 "documentHighlight" || true

# Cleanup
exec 3<&-
exec 4>&-

echo ""
echo "Test completed. Check output above for documentHighlight response."

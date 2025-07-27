#!/bin/bash
# Simple LSP test with proper message format

# Create proper LSP message
MSG='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
LEN=${#MSG}

# Send to LSP server
(
  printf "Content-Length: %d\r\n\r\n%s" "$LEN" "$MSG"
  sleep 1
) | timeout 3s ./target/release/perl-lsp --stdio 2>&1
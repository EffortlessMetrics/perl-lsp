#!/usr/bin/env python3
"""Minimal test for the Perl LSP server."""

import json
import sys

def create_lsp_message(content):
    """Create a proper LSP message with headers."""
    content_str = json.dumps(content)
    content_bytes = content_str.encode('utf-8')
    header = f"Content-Length: {len(content_bytes)}\r\n\r\n"
    return header.encode('utf-8') + content_bytes

# Create initialize request
init_request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "processId": None,
        "rootUri": "file:///tmp",
        "capabilities": {}
    }
}

# Send to stdout (which will be piped to LSP server)
sys.stdout.buffer.write(create_lsp_message(init_request))
sys.stdout.buffer.flush()

# Also create a shutdown request
shutdown_request = {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "shutdown"
}

sys.stdout.buffer.write(create_lsp_message(shutdown_request))
sys.stdout.buffer.flush()
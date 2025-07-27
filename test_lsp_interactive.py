#!/usr/bin/env python3
"""Interactive test for the Perl LSP server."""

import subprocess
import json
import time

def create_lsp_message(obj):
    """Create a proper LSP message with headers."""
    content = json.dumps(obj)
    content_bytes = content.encode('utf-8')
    header = f"Content-Length: {len(content_bytes)}\r\n\r\n"
    return (header + content).encode('utf-8')

def read_lsp_message(proc):
    """Read an LSP message from the server."""
    headers = {}
    while True:
        line = proc.stdout.readline().decode('utf-8').rstrip('\r\n')
        if not line:
            break
        key, value = line.split(': ', 1)
        headers[key] = value
    
    if 'Content-Length' in headers:
        length = int(headers['Content-Length'])
        content = proc.stdout.read(length).decode('utf-8')
        return json.loads(content)
    return None

# Start the LSP server
print("Starting LSP server...")
proc = subprocess.Popen(
    ['./target/release/perl-lsp', '--stdio'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    bufsize=0
)

try:
    # Send initialize request
    print("\nSending initialize request...")
    init_req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": None,
            "rootUri": None,
            "capabilities": {}
        }
    }
    proc.stdin.write(create_lsp_message(init_req))
    proc.stdin.flush()
    
    # Give server time to respond
    time.sleep(0.5)
    
    # Check if we have output
    import select
    if select.select([proc.stdout], [], [], 0.1)[0]:
        response = read_lsp_message(proc)
        print(f"Initialize response: {json.dumps(response, indent=2)}")
    
    # Send initialized notification
    print("\nSending initialized notification...")
    proc.stdin.write(create_lsp_message({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }))
    proc.stdin.flush()
    
    # Open a document
    print("\nOpening document...")
    doc_content = """#!/usr/bin/env perl
use strict;
use warnings;

my $name = "World";
print "Hello, $name!\\n";

# This should trigger a diagnostic
$undefined_var = 42;
"""
    
    proc.stdin.write(create_lsp_message({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/test.pl",
                "languageId": "perl",
                "version": 1,
                "text": doc_content
            }
        }
    }))
    proc.stdin.flush()
    
    # Request completion
    print("\nRequesting completion...")
    proc.stdin.write(create_lsp_message({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///tmp/test.pl"},
            "position": {"line": 4, "character": 5}
        }
    }))
    proc.stdin.flush()
    
    # Give time for response
    time.sleep(0.5)
    
    if select.select([proc.stdout], [], [], 0.1)[0]:
        response = read_lsp_message(proc)
        print(f"Completion response: {json.dumps(response, indent=2) if response else 'No response'}")
    
    # Shutdown
    print("\nShutting down...")
    proc.stdin.write(create_lsp_message({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown"
    }))
    proc.stdin.flush()
    
    time.sleep(0.5)
    if select.select([proc.stdout], [], [], 0.1)[0]:
        response = read_lsp_message(proc)
        print(f"Shutdown response: {response}")
    
    # Check stderr
    stderr = proc.stderr.read().decode('utf-8')
    if stderr:
        print(f"\nServer stderr:\n{stderr}")
    
finally:
    proc.terminate()
    proc.wait()
    print("\nTest complete!")
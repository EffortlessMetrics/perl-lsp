#!/usr/bin/env python3
"""
Test script for the Perl LSP server.
This script sends LSP requests and displays the responses.
"""

import json
import subprocess
import sys
import os
from pathlib import Path

def send_request(proc, request):
    """Send a JSON-RPC request to the LSP server."""
    content = json.dumps(request)
    header = f"Content-Length: {len(content)}\r\n\r\n"
    message = header + content
    
    proc.stdin.write(message.encode())
    proc.stdin.flush()
    
    # Read response
    content_length = 0
    while True:
        line = proc.stdout.readline().decode().strip()
        if line.startswith("Content-Length:"):
            content_length = int(line.split(":")[1].strip())
        elif line == "":
            break
    
    if content_length > 0:
        response = proc.stdout.read(content_length).decode()
        return json.loads(response)
    return None

def main():
    # Path to the test file
    test_file = Path(__file__).parent / "lsp_demo.pl"
    test_file_uri = f"file://{test_file.absolute()}"
    
    # Start the LSP server
    lsp_binary = Path(__file__).parent.parent.parent.parent / "target" / "debug" / "perl-lsp"
    if not lsp_binary.exists():
        print(f"LSP binary not found at {lsp_binary}")
        print("Please build it first: cargo build -p perl-parser --bin perl-lsp")
        sys.exit(1)
    
    print(f"Starting LSP server: {lsp_binary}")
    proc = subprocess.Popen(
        [str(lsp_binary)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    
    try:
        # Initialize
        print("\n1. Sending initialize request...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": os.getpid(),
                "rootUri": f"file://{Path.cwd()}",
                "capabilities": {}
            }
        })
        print(f"Response: {json.dumps(response, indent=2)}")
        
        # Open document
        print(f"\n2. Opening document: {test_file}")
        with open(test_file) as f:
            content = f.read()
        
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": test_file_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        })
        
        # Wait a bit for diagnostics
        import time
        time.sleep(0.5)
        
        # Request diagnostics (they should be pushed automatically)
        print("\n3. Diagnostics should appear in stderr...")
        
        # Request completion at a position
        print("\n4. Requesting completion after 'my $'...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": test_file_uri},
                "position": {"line": 5, "character": 5}  # After "my $"
            }
        })
        if response and "result" in response:
            print(f"Completions: {len(response['result'])} items")
            for item in response['result'][:5]:  # Show first 5
                print(f"  - {item['label']}: {item.get('detail', '')}")
        
        # Request hover
        print("\n5. Requesting hover over 'greet' function...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": test_file_uri},
                "position": {"line": 17, "character": 5}  # On "greet"
            }
        })
        if response and "result" in response:
            print(f"Hover: {response['result']}")
        
        # Request code actions
        print("\n6. Requesting code actions for undeclared variable...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {"uri": test_file_uri},
                "range": {
                    "start": {"line": 14, "character": 0},
                    "end": {"line": 14, "character": 20}
                }
            }
        })
        if response and "result" in response:
            print(f"Code actions: {len(response['result'])} available")
            for action in response['result']:
                print(f"  - {action['title']}")
        
        # Go to definition
        print("\n7. Requesting definition of 'greet' function call...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": test_file_uri},
                "position": {"line": 24, "character": 0}  # On "greet" call
            }
        })
        if response and "result" in response:
            print(f"Definition: {response['result']}")
        
        # Shutdown
        print("\n8. Shutting down...")
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "shutdown"
        })
        print(f"Shutdown response: {response}")
        
    finally:
        # Check stderr for any errors
        stderr = proc.stderr.read().decode()
        if stderr:
            print(f"\nServer stderr output:\n{stderr}")
        
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    main()
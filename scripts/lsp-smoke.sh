#!/usr/bin/env bash
set -euo pipefail

BIN=${BIN:-target/debug/perl-lsp}
cargo build -p perl-parser --bin perl-lsp --quiet

python3 - "$BIN" <<'PY'
import json, subprocess, sys, os, time

def frame(obj):
    b = json.dumps(obj).encode()
    return b"Content-Length: %d\r\n\r\n" % len(b) + b

proc = subprocess.Popen(
    [sys.argv[1], "--stdio"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
)

def send(obj): proc.stdin.write(frame(obj)); proc.stdin.flush()
def recv():
    # read header
    hdr = b""
    while not hdr.endswith(b"\r\n\r\n"):
        b = proc.stdout.read(1)
        if not b: raise SystemExit("EOF")
        hdr += b
    length = None
    for line in hdr.split(b"\r\n"):
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":",1)[1])
    body = proc.stdout.read(length)
    return json.loads(body)

def recv_response(expected_id):
    # Keep receiving until we get a response with the expected ID
    while True:
        msg = recv()
        if "id" in msg and msg["id"] == expected_id:
            return msg
        # Otherwise it's a notification, ignore it

# 1) initialize
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
_ = recv_response(1)

# 2) didOpen with simple inheritance and a repeated variable
text = "package Base; package Child; use parent 'Base'; my $x=1; $x++;\n"
send({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
    "textDocument":{"uri":"file:///test.pl","languageId":"perl","version":1,"text":text}
}})

# 3) documentHighlight on $x (position 52 = first $x)
send({"jsonrpc":"2.0","id":2,"method":"textDocument/documentHighlight","params":{
    "textDocument":{"uri":"file:///test.pl"},"position":{"line":0,"character":52}
}})
hl = recv_response(2)
assert "result" in hl, f"No result in response: {hl}"
assert isinstance(hl["result"], list) and len(hl["result"]) >= 2, f"Expected 2+ highlights, got {hl.get('result')}"

# 4) prepareTypeHierarchy on Base
send({"jsonrpc":"2.0","id":3,"method":"textDocument/prepareTypeHierarchy","params":{
    "textDocument":{"uri":"file:///test.pl"},"position":{"line":0,"character":10}
}})
prep = recv_response(3)
assert "result" in prep and prep["result"], f"No prepare result: {prep}"

# 5) typeHierarchy/subtypes
item = prep["result"][0]
send({"jsonrpc":"2.0","id":4,"method":"typeHierarchy/subtypes","params":{"item":item}})
subs = recv_response(4)
assert any(i.get("name") == "Child" for i in (subs["result"] or [])), "Child subtype not found"

print("OK: documentHighlight + typeHierarchy")
proc.terminate()
PY
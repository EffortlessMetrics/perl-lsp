#!/bin/bash
# Test document lifecycle to isolate memory leaks
# Runs multiple iterations of open/close to see if memory returns to baseline

set -e

WORKDIR=$(mktemp -d)

echo "=== Testing Document Lifecycle Memory ===" >&2
echo "Each iteration: open 50 files, change each 5 times, close all" >&2
echo "Monitor RSS growth between iterations" >&2

cat > "$WORKDIR/harness.py" <<'EOF'
#!/usr/bin/env python3
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote

def uri(p: pathlib.Path) -> str:
    return "file://" + quote(str(p))

def write_msg(fp, obj):
    body = json.dumps(obj).encode("utf-8")
    fp.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
    fp.write(body)
    fp.flush()

def read_msg(fp):
    headers = {}
    while True:
        line = fp.readline()
        if not line:
            return None
        if line == b"\r\n":
            break
        k, v = line.decode("ascii").split(":", 1)
        headers[k.lower()] = v.strip()
    length = int(headers["content-length"])
    return json.loads(fp.read(length).decode("utf-8"))

def rss_kb(pid: int) -> int:
    try:
        out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True)
        return int(out.strip())
    except:
        return 0

def make_file(root: pathlib.Path, i: int) -> pathlib.Path:
    p = root / f"file_{i}.pl"
    p.write_text(f"package P{i};\nsub f{i} {{ return {i}; }}\n1;\n")
    return p

ROOT = pathlib.Path(tempfile.mkdtemp(prefix="doc-lifecycle-"))
N_ITERATIONS = 5
N_FILES = 50
N_CHANGES = 5

proc = subprocess.Popen(
    ["./target/release/perl-lsp", "--stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
)
pid = proc.pid

# Initialize
write_msg(proc.stdin, {
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {"processId": None, "rootUri": uri(ROOT), "capabilities": {}}
})
_ = read_msg(proc.stdout)
write_msg(proc.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

print(f"i,iteration,rss_kb", file=sys.stderr)
for iteration in range(N_ITERATIONS):
    files = [make_file(ROOT / f"iter{iteration}", i) for i in range(N_FILES)]

    for i, p in enumerate(files):
        text = p.read_text()
        write_msg(proc.stdin, {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri(p), "languageId": "perl", "version": 1, "text": text}}
        })

        for v in range(2, N_CHANGES + 2):
            text += f"\nmy $v{v} = {v};\n"
            write_msg(proc.stdin, {
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {"textDocument": {"uri": uri(p), "version": v}, "contentChanges": [{"text": text}]}
            })

        write_msg(proc.stdin, {
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {"textDocument": {"uri": uri(p)}}
        })

    time.sleep(1)
    rss = rss_kb(pid)
    print(f"{iteration},{iteration},{rss}", file=sys.stderr)

proc.terminate()
EOF

python3 "$WORKDIR/harness.py"
echo "Done" >&2

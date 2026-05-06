#!/usr/bin/env python3
"""LSP stdio storm harness for memory leak investigation.

Starts perllsp --stdio, initializes a workspace, opens files, mutates them,
optionally issues workspace/symbol queries, then closes them while sampling RSS.

Usage:
    N_FILES=200 N_CHANGES=20 DO_WORKSPACE_SYMBOL=1 python3 repro_lsp_storm.py 2>samples.log
"""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote

ROOT = pathlib.Path(tempfile.mkdtemp(prefix="perl-lsp-leak-"))
N_FILES = int(os.environ.get("N_FILES", "200"))
N_CHANGES = int(os.environ.get("N_CHANGES", "20"))
DO_WORKSPACE_SYMBOL = os.environ.get("DO_WORKSPACE_SYMBOL", "1") == "1"
BINARY = os.environ.get("BINARY", "./target/release/perl-lsp")


def uri(p: pathlib.Path) -> str:
    """Convert path to LSP file:// URI."""
    return "file://" + quote(str(p))


def write_msg(fp, obj):
    """Write LSP message to process stdin."""
    body = json.dumps(obj).encode("utf-8")
    fp.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii"))
    fp.write(body)
    fp.flush()


def read_msg(fp):
    """Read LSP message from process stdout."""
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
    """Get RSS memory for a process in KB."""
    try:
        out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True)
        return int(out.strip())
    except (subprocess.CalledProcessError, ValueError):
        return 0


def make_file(i: int) -> pathlib.Path:
    """Create a synthetic Perl module."""
    p = ROOT / f"file_{i}.pl"
    p.write_text(
        "package Leak::Pkg::{i};\n"
        "use strict;\nuse warnings;\n"
        "sub f_{i} {{ my ($x) = @_; return $x + 1; }}\n".format(i=i),
        encoding="utf-8",
    )
    return p


print(f"[info] Workspace root: {ROOT}", file=sys.stderr)
print(f"[info] Files: {N_FILES}, Changes per file: {N_CHANGES}, Workspace symbol: {DO_WORKSPACE_SYMBOL}", file=sys.stderr)

# Start server
proc = subprocess.Popen(
    [BINARY, "--stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=open("/tmp/perllsp.stderr.log", "wb"),
)
assert proc.stdin and proc.stdout
pid = proc.pid
print(f"[info] Started perllsp pid={pid}", file=sys.stderr)

# Initialize
write_msg(proc.stdin, {
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {
        "processId": None,
        "rootUri": uri(ROOT),
        "capabilities": {}
    }
})
init_resp = read_msg(proc.stdout)
print(f"[init] response id={init_resp.get('id')}", file=sys.stderr)

write_msg(proc.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

# Create synthetic files
files = [make_file(i) for i in range(N_FILES)]
samples = []

# Open, change, query, close
for i, p in enumerate(files):
    text = p.read_text(encoding="utf-8")
    write_msg(proc.stdin, {
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri(p),
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        }
    })

    for v in range(2, N_CHANGES + 2):
        text += f"\nmy $v{v} = {v};\n"
        write_msg(proc.stdin, {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri(p), "version": v},
                "contentChanges": [{"text": text}]
            }
        })

    if DO_WORKSPACE_SYMBOL and i % 20 == 0:
        req_id = 10000 + i
        write_msg(proc.stdin, {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "workspace/symbol",
            "params": {"query": "f_"}
        })
        _ = read_msg(proc.stdout)

    write_msg(proc.stdin, {
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {"textDocument": {"uri": uri(p)}}
    })

    if i % 10 == 0:
        rss = rss_kb(pid)
        samples.append((time.time(), i, rss))
        print(f"sample i={i} files_closed={i} rss_kb={rss}", file=sys.stderr)

# Wait for final stabilization
time.sleep(3)
rss = rss_kb(pid)
samples.append((time.time(), N_FILES, rss))
print(f"final i={N_FILES} files_closed={N_FILES} rss_kb={rss}", file=sys.stderr)

# Emit summary
print("\n[samples] (timestamp, file_idx, rss_kb):", file=sys.stderr)
for t, idx, rss in samples:
    print(f"  {t:.2f}  idx={idx:>4}  rss={rss:>8} KB", file=sys.stderr)

# Calculate slope over final 80% of samples
if len(samples) >= 5:
    start_idx = len(samples) // 5
    final_samples = samples[start_idx:]
    if len(final_samples) >= 2:
        t0, _, rss0 = final_samples[0]
        t1, _, rss1 = final_samples[-1]
        dt = t1 - t0
        drss = rss1 - rss0
        if dt > 0:
            slope = drss / dt
            print(f"\n[analysis] final 80% slope: {slope:.2f} KB/s", file=sys.stderr)
            print(f"[analysis] final rss: {rss1} KB, initial rss: {samples[0][2]} KB", file=sys.stderr)
        else:
            print("\n[analysis] insufficient time between final samples", file=sys.stderr)

proc.terminate()
print(f"\n[done] Terminated perllsp pid={pid}", file=sys.stderr)

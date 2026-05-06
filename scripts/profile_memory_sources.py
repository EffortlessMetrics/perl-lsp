#!/usr/bin/env python3
"""Profile which component is accumulating memory during document churn.

Runs LSP server with specific cleanup disabled to isolate the leak source.
"""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote

ROOT = pathlib.Path(tempfile.mkdtemp(prefix="perl-lsp-profile-"))
N_FILES = int(os.environ.get("N_FILES", "100"))
N_CHANGES = int(os.environ.get("N_CHANGES", "5"))

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

def rss_mb(pid: int) -> float:
    try:
        out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True)
        return int(out.strip()) / 1024.0
    except:
        return 0.0

def make_file(i: int) -> pathlib.Path:
    p = ROOT / f"file_{i}.pl"
    p.write_text(
        f"package P{i};\n"
        f"use strict;\nuse warnings;\n"
        f"sub f{i} {{ return {i}; }}\n"
        f"1;\n",
        encoding="utf-8",
    )
    return p

print(f"[info] Profiling memory sources with {N_FILES} files, {N_CHANGES} changes/file")
print(f"[info] Workspace: {ROOT}")

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

time.sleep(0.5)
baseline_mb = rss_mb(pid)
print(f"\nBaseline: {baseline_mb:.1f} MB")
print(f"File#\tRSS_MB\tDelta_MB\tRate_KB/file")

files = [make_file(i) for i in range(N_FILES)]
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

    if i % 10 == 0:
        time.sleep(0.1)
        current_mb = rss_mb(pid)
        delta_mb = current_mb - baseline_mb
        rate = (delta_mb * 1024) / max(i, 1) if i > 0 else 0
        print(f"{i}\t{current_mb:.1f}\t{delta_mb:.1f}\t{rate:.0f}")

time.sleep(1)
final_mb = rss_mb(pid)
total_delta = final_mb - baseline_mb
per_file = (total_delta * 1024) / N_FILES

print(f"\nFinal: {final_mb:.1f} MB")
print(f"Total delta: {total_delta:.1f} MB ({total_delta * 1024:.0f} KB)")
print(f"Per-file leak: {per_file:.1f} KB")

proc.terminate()

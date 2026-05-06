#!/usr/bin/env python3
"""Drive perl-lsp through open/change/close churn and record RSS samples."""

from __future__ import annotations

import argparse
import csv
import json
import os
import queue
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from urllib.parse import quote


def env_int(name: str, default: int) -> int:
    value = os.environ.get(name)
    if value is None:
        return default
    try:
        return int(value)
    except ValueError:
        raise SystemExit(f"{name} must be an integer, got {value!r}")


def file_uri(path: Path) -> str:
    return "file://" + quote(str(path.resolve()))


def rss_kb(pid: int) -> int:
    status = Path(f"/proc/{pid}/status")
    if status.exists():
        for line in status.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1])

    out = subprocess.check_output(["ps", "-o", "rss=", "-p", str(pid)], text=True)
    return int(out.strip() or "0")


def send_message(proc: subprocess.Popen[bytes], payload: dict) -> None:
    body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
    assert proc.stdin is not None
    proc.stdin.write(header + body)
    proc.stdin.flush()


def read_messages(proc: subprocess.Popen[bytes], out: "queue.Queue[dict]") -> None:
    assert proc.stdout is not None
    stream = proc.stdout
    while True:
        headers: dict[str, str] = {}
        while True:
            line = stream.readline()
            if not line:
                return
            if line == b"\r\n":
                break
            key, _, value = line.decode("ascii", errors="replace").partition(":")
            headers[key.lower()] = value.strip()
        length = int(headers.get("content-length", "0"))
        if length <= 0:
            continue
        raw = stream.read(length)
        if not raw:
            return
        try:
            out.put(json.loads(raw.decode("utf-8")))
        except json.JSONDecodeError:
            continue


def perl_source(index: int, version: int) -> str:
    package = f"Storm::File{index:05d}"
    return (
        f"package {package};\n"
        "use strict;\n"
        "use warnings;\n\n"
        f"sub value_{index:05d} {{ return {index + version}; }}\n"
        f"sub call_{index:05d} {{ return value_{index:05d}(); }}\n\n"
        "1;\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--csv-out", type=Path)
    parser.add_argument("--binary", default=os.environ.get("BINARY", "./target/release/perl-lsp"))
    parser.add_argument("--n-files", type=int, default=env_int("N_FILES", 500))
    parser.add_argument("--n-changes", type=int, default=env_int("N_CHANGES", 10))
    parser.add_argument(
        "--workspace-symbol",
        action="store_true",
        default=os.environ.get("DO_WORKSPACE_SYMBOL", "0") == "1",
    )
    parser.add_argument(
        "--delete-after-close",
        action="store_true",
        default=os.environ.get("DELETE_AFTER_CLOSE", "0") == "1",
        help="unlink each file after didClose and send a watched-file DELETED event",
    )
    parser.add_argument("--sample-every", type=int, default=env_int("SAMPLE_EVERY", 10))
    parser.add_argument("--settle-seconds", type=float, default=float(os.environ.get("SETTLE_SECONDS", "1.0")))
    parser.add_argument("--server-stderr", type=Path)
    args = parser.parse_args()

    binary = shutil.which(args.binary) or args.binary
    server_stderr = subprocess.DEVNULL
    stderr_handle = None
    if args.server_stderr:
        args.server_stderr.parent.mkdir(parents=True, exist_ok=True)
        stderr_handle = args.server_stderr.open("wb")
        server_stderr = stderr_handle

    with tempfile.TemporaryDirectory(prefix="perl-lsp-storm-") as tmp:
        root = Path(tmp)
        proc = subprocess.Popen(
            [binary, "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=server_stderr,
        )
        responses: "queue.Queue[dict]" = queue.Queue()
        reader = threading.Thread(target=read_messages, args=(proc, responses), daemon=True)
        reader.start()

        next_id = 1
        start = time.monotonic()
        samples: list[dict] = []

        def sample(phase: str, file_index: int) -> None:
            try:
                rss = rss_kb(proc.pid)
            except Exception as exc:  # pragma: no cover - platform fallback
                print(f"rss sample failed: {exc}", file=sys.stderr)
                rss = 0
            row = {
                "elapsed_s": round(time.monotonic() - start, 3),
                "phase": phase,
                "file_index": file_index,
                "rss_kb": rss,
            }
            samples.append(row)
            print(json.dumps(row, separators=(",", ":")), file=sys.stderr)

        def request(method: str, params: dict | None = None) -> int:
            nonlocal next_id
            req_id = next_id
            next_id += 1
            send_message(proc, {"jsonrpc": "2.0", "id": req_id, "method": method, "params": params or {}})
            return req_id

        def notify(method: str, params: dict | None = None) -> None:
            send_message(proc, {"jsonrpc": "2.0", "method": method, "params": params or {}})

        def wait_for_response(req_id: int, timeout: float = 10.0) -> dict:
            deadline = time.monotonic() + timeout
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError(f"timed out waiting for response id {req_id}")
                try:
                    message = responses.get(timeout=remaining)
                except queue.Empty as exc:
                    raise TimeoutError(f"timed out waiting for response id {req_id}") from exc
                if message.get("id") == req_id:
                    return message

        try:
            initialize_id = request(
                "initialize",
                {
                    "processId": os.getpid(),
                    "rootUri": file_uri(root),
                    "capabilities": {
                        "textDocument": {
                            "synchronization": {"didSave": True},
                            "publishDiagnostics": {"relatedInformation": True},
                        },
                        "workspace": {"workspaceFolders": True},
                    },
                    "workspaceFolders": [{"uri": file_uri(root), "name": "storm"}],
                },
            )
            wait_for_response(initialize_id, timeout=10.0)
            notify("initialized", {})
            sample("initialized", 0)

            for i in range(args.n_files):
                path = root / f"File{i:05d}.pm"
                text = perl_source(i, 0)
                path.write_text(text, encoding="utf-8")
                uri = file_uri(path)
                notify(
                    "textDocument/didOpen",
                    {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "perl",
                            "version": 1,
                            "text": text,
                        }
                    },
                )
                for change in range(args.n_changes):
                    text = perl_source(i, change + 1)
                    notify(
                        "textDocument/didChange",
                        {
                            "textDocument": {"uri": uri, "version": change + 2},
                            "contentChanges": [{"text": text}],
                        },
                    )
                if args.workspace_symbol:
                    request("workspace/symbol", {"query": f"value_{i:05d}"})
                notify("textDocument/didClose", {"textDocument": {"uri": uri}})
                if args.delete_after_close:
                    path.unlink(missing_ok=True)
                    notify(
                        "workspace/didChangeWatchedFiles",
                        {
                            "changes": [
                                {
                                    "uri": uri,
                                    "type": 3,
                                }
                            ]
                        },
                    )

                if i % max(args.sample_every, 1) == 0 or i == args.n_files - 1:
                    sample("churn", i + 1)

            time.sleep(args.settle_seconds)
            sample("settled", args.n_files)
            shutdown_id = request("shutdown", {})
            wait_for_response(shutdown_id, timeout=10.0)
            notify("exit", {})
        finally:
            try:
                if proc.stdin:
                    proc.stdin.close()
            except BrokenPipeError:
                pass
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5)
            if stderr_handle:
                stderr_handle.close()

        result = {
            "schema": 1,
            "binary": str(binary),
            "n_files": args.n_files,
            "n_changes": args.n_changes,
            "workspace_symbol": args.workspace_symbol,
            "delete_after_close": args.delete_after_close,
            "sample_every": args.sample_every,
            "settle_seconds": args.settle_seconds,
            "samples": samples,
        }

        if args.json_out:
            args.json_out.parent.mkdir(parents=True, exist_ok=True)
            args.json_out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        if args.csv_out:
            args.csv_out.parent.mkdir(parents=True, exist_ok=True)
            with args.csv_out.open("w", newline="", encoding="utf-8") as fh:
                writer = csv.DictWriter(fh, fieldnames=["elapsed_s", "phase", "file_index", "rss_kb"])
                writer.writeheader()
                writer.writerows(samples)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

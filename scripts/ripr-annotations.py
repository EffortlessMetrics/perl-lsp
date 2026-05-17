#!/usr/bin/env python3
"""Emit non-blocking GitHub annotations from RIPR line-placeable comments."""

import json
from pathlib import Path

path = Path("target/ripr/review/comments.json")
if not path.exists():
    raise SystemExit(0)

data = json.loads(path.read_text(encoding="utf-8"))

for item in data.get("comments", []):
    file = item.get("path") or item.get("file")
    line = item.get("line")
    title = item.get("title") or "RIPR"
    body = item.get("body") or item.get("message") or ""

    if not file or not line:
        continue

    body = str(body).replace("\n", "%0A")
    print(f"::warning file={file},line={line},title={title}::{body}")

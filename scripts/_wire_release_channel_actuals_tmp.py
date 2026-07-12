#!/usr/bin/env python3
"""One-shot wiring of release-channel actuals into the checked-in drift gate."""

from pathlib import Path

path = Path("scripts/check_release_history.sh")
text = path.read_text(encoding="utf-8")
old = '''# ── Exit ──────────────────────────────────────────────────────────────────────

if [[ "$DRIFT_FOUND" -eq 1 ]]; then
'''
new = '''# ── Check 6: Verified channel actuals must not regress ─────────────────────────

if ! python3 scripts/check_release_channel_actuals.py; then
    error "Release-channel actuals drift check failed"
fi

# ── Exit ──────────────────────────────────────────────────────────────────────

if [[ "$DRIFT_FOUND" -eq 1 ]]; then
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"release history exit marker: expected one match, found {count}")
path.write_text(text.replace(old, new, 1), encoding="utf-8")

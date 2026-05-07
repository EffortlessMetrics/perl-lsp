#!/usr/bin/env python3
"""Aggregate ci-actuals.json artifacts into a per-lane percentile history.

Inputs (one or more):
  --actuals-dir DIR      Walk DIR for ci-actuals.json files (default: target/ci/actuals/).
  --window-days N        Only consider receipts newer than N days (default: 14).
  --output PATH          Write history JSON here (default: .ci/metrics/ci-lane-history.json).
  --static-lanes PATH    policy/ci-lanes.toml for static_floor reference.

Output: .ci/metrics/ci-lane-history.json with per-lane sample counts and
p50/p90/p95. PR 13's planner can consume this to derive learned estimates
once enough samples accumulate.

Schema is intentionally tolerant: lanes with fewer than MIN_SAMPLES
samples are still recorded but flagged with `learned: false` so the
planner falls back to the static floor.
"""
from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
import time
import tomllib
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
MIN_SAMPLES_FOR_LEARNED = 5


def percentile(values: list[float], p: float) -> float:
    """Linear-interpolation percentile, p in [0, 100]."""
    if not values:
        return 0.0
    if len(values) == 1:
        return values[0]
    sorted_vals = sorted(values)
    k = (len(sorted_vals) - 1) * (p / 100.0)
    lo = int(math.floor(k))
    hi = int(math.ceil(k))
    if lo == hi:
        return sorted_vals[lo]
    return sorted_vals[lo] + (sorted_vals[hi] - sorted_vals[lo]) * (k - lo)


def collect_actuals(
    *, actuals_dir: Path, window_days: int
) -> dict[str, list[float]]:
    """Walk actuals_dir for ci-actuals.json files, return per-lane LEM samples."""
    samples: dict[str, list[float]] = {}
    if not actuals_dir.exists():
        return samples

    cutoff = time.time() - window_days * 86400
    for path in sorted(actuals_dir.rglob("*.json")):
        try:
            mtime = path.stat().st_mtime
        except OSError:
            continue
        if mtime < cutoff:
            continue
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            continue
        if not isinstance(doc, dict):
            continue
        for job in doc.get("jobs", []):
            if not isinstance(job, dict):
                continue
            lane_id = job.get("gate_name") or job.get("lane_id")
            actual = job.get("actual_lem")
            if not lane_id or not isinstance(actual, (int, float)):
                continue
            samples.setdefault(lane_id, []).append(float(actual))
    return samples


def static_floors(lanes_toml: Path) -> dict[str, float]:
    if not lanes_toml.exists():
        return {}
    doc = tomllib.loads(lanes_toml.read_text(encoding="utf-8"))
    out: dict[str, float] = {}
    for lane_id, lane in doc.get("lane", {}).items():
        base = lane.get("base_lem")
        if isinstance(base, (int, float)):
            out[lane_id] = float(base)
    return out


def build_history(
    *, samples: dict[str, list[float]], floors: dict[str, float], window_days: int
) -> dict[str, Any]:
    lanes_out: dict[str, Any] = {}
    # Include every lane known to policy, even when samples are empty: planner
    # readers can iterate the full keyspace without checking presence.
    for lane_id in sorted(set(samples.keys()) | set(floors.keys())):
        s = samples.get(lane_id, [])
        floor = floors.get(lane_id)
        learned = len(s) >= MIN_SAMPLES_FOR_LEARNED
        entry: dict[str, Any] = {
            "samples": len(s),
            "static_floor": floor,
            "learned": learned,
        }
        if s:
            entry.update(
                {
                    "p50": percentile(s, 50),
                    "p90": percentile(s, 90),
                    "p95": percentile(s, 95),
                    "min": min(s),
                    "max": max(s),
                    "mean": statistics.fmean(s),
                }
            )
        lanes_out[lane_id] = entry

    return {
        "schema_version": SCHEMA_VERSION,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "window_days": window_days,
        "min_samples_for_learned": MIN_SAMPLES_FOR_LEARNED,
        "lane_count": len(lanes_out),
        "lanes": lanes_out,
    }


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--actuals-dir", type=Path, default=Path("target/ci/actuals"),
        help="Directory to walk for ci-actuals.json artifacts.",
    )
    p.add_argument("--window-days", type=int, default=14)
    p.add_argument(
        "--output", type=Path,
        default=Path(".ci/metrics/ci-lane-history.json"),
    )
    p.add_argument(
        "--static-lanes", type=Path, default=Path("policy/ci-lanes.toml")
    )
    args = p.parse_args()

    samples = collect_actuals(
        actuals_dir=args.actuals_dir, window_days=args.window_days
    )
    floors = static_floors(args.static_lanes)
    history = build_history(
        samples=samples, floors=floors, window_days=args.window_days
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(history, indent=2) + "\n", encoding="utf-8"
    )

    learned = sum(1 for entry in history["lanes"].values() if entry["learned"])
    print(
        json.dumps(
            {
                "lanes": history["lane_count"],
                "learned": learned,
                "window_days": args.window_days,
            }
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

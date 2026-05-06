#!/usr/bin/env python3
"""Assert that RSS samples flatten after warmup."""

from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path


def median_slope(points: list[tuple[int, int]]) -> float:
    slopes: list[float] = []
    for idx, (x1, y1) in enumerate(points):
        for x2, y2 in points[idx + 1 :]:
            dx = x2 - x1
            if dx > 0:
                slopes.append((y2 - y1) / dx)
    return statistics.median(slopes) if slopes else 0.0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("json_file", type=Path)
    parser.add_argument("--warmup-fraction", type=float, default=0.5)
    parser.add_argument("--max-tail-growth-kb", type=int, default=8192)
    parser.add_argument("--max-tail-growth-pct", type=float, default=8.0)
    parser.add_argument("--max-slope-kb-per-file", type=float, default=2.0)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--loose", action="store_true")
    args = parser.parse_args()

    if args.loose:
        args.max_tail_growth_kb *= 2
        args.max_tail_growth_pct *= 2.0
        args.max_slope_kb_per_file *= 2.0

    payload = json.loads(args.json_file.read_text(encoding="utf-8"))
    samples = [
        sample
        for sample in payload.get("samples", [])
        if sample.get("phase") in {"churn", "settled"} and int(sample.get("rss_kb", 0)) > 0
    ]
    if len(samples) < 4:
        raise SystemExit(f"need at least 4 RSS samples, got {len(samples)}")

    start = max(0, min(len(samples) - 2, int(len(samples) * args.warmup_fraction)))
    tail = samples[start:]
    tail_points = [(int(s["file_index"]), int(s["rss_kb"])) for s in tail]
    tail_rss = [rss for _, rss in tail_points]
    tail_min = min(tail_rss)
    tail_last = tail_rss[-1]
    tail_growth_kb = tail_last - tail_min
    tail_growth_pct = (tail_growth_kb / max(tail_min, 1)) * 100.0
    slope = median_slope(tail_points)
    material_growth_kb = max(2048, args.max_tail_growth_kb // 4)

    failures: list[str] = []
    if tail_growth_kb > args.max_tail_growth_kb and tail_growth_pct > args.max_tail_growth_pct:
        failures.append(
            "tail growth exceeded limits: "
            f"{tail_growth_kb} KB ({tail_growth_pct:.2f}%)"
        )
    if tail_growth_kb > material_growth_kb and slope > args.max_slope_kb_per_file:
        failures.append(
            "tail slope exceeded limit: "
            f"{slope:.2f} KB/file > {args.max_slope_kb_per_file:.2f} KB/file"
        )

    summary = {
        "json_file": str(args.json_file),
        "samples": len(samples),
        "tail_samples": len(tail),
        "tail_min_kb": tail_min,
        "tail_last_kb": tail_last,
        "tail_growth_kb": tail_growth_kb,
        "tail_growth_pct": round(tail_growth_pct, 3),
        "median_tail_slope_kb_per_file": round(slope, 3),
        "material_growth_kb": material_growth_kb,
        "passed": not failures,
    }
    print(json.dumps(summary, indent=2))
    if args.summary_out:
        args.summary_out.parent.mkdir(parents=True, exist_ok=True)
        args.summary_out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    if failures:
        raise SystemExit("; ".join(failures))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

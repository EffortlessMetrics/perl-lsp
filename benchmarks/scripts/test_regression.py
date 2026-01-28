#!/usr/bin/env python3
"""Create simulated benchmark regression for testing alerts."""

import json
import sys
from pathlib import Path

def create_regression(baseline_file: Path, output_file: Path, regression_pct: float = 25):
    """Create a simulated regression by increasing benchmark times."""
    with open(baseline_file) as f:
        data = json.load(f)

    # Simulate regression in parser benchmarks
    if 'results' in data and 'parser' in data['results']:
        for bench_name in data['results']['parser']:
            if bench_name != '_total_time':
                original = data['results']['parser'][bench_name]['mean_ns']
                data['results']['parser'][bench_name]['mean_ns'] = int(original * (1 + regression_pct / 100))
                print(f"Regressed {bench_name}: +{regression_pct}%")
                break

    # Write to output
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)

    print(f"Created simulated regression at {output_file}")

if __name__ == "__main__":
    baseline = Path("benchmarks/baselines/v0.9.0.json")
    output = Path("/tmp/regressed_benchmark.json")
    create_regression(baseline, output, regression_pct=25)

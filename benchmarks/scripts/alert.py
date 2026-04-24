#!/usr/bin/env python3
"""Generate performance regression alerts for PR comments.

This script reads benchmark comparison results and threshold configuration,
then generates formatted alerts for GitHub PR comments.

Usage:
    ./alert.py benchmarks/results/latest.json
    ./alert.py --config .ci/benchmark-thresholds.yaml
    ./alert.py --format markdown > alert.md
    ./alert.py --check  # Exit non-zero if critical regression
"""

import argparse
import json
import re
import sys
import yaml
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Any

# ANSI colors
RED = '\033[0;31m'
GREEN = '\033[0;32m'
YELLOW = '\033[1;33m'
BLUE = '\033[0;34m'
NC = '\033[0m'

CATEGORY_ALIASES = {
    "workspace_index": "index",
}

UNIT_MULTIPLIERS = {
    "ns": 1,
    "us": 1_000,
    "µs": 1_000,
    "μs": 1_000,
    "ms": 1_000_000,
    "s": 1_000_000_000,
}


def load_config(config_path: Path) -> dict:
    """Load threshold configuration."""
    with open(config_path) as f:
        return yaml.safe_load(f)


def load_json(path: Path) -> dict:
    """Load JSON file."""
    with open(path) as f:
        return json.load(f)


def normalize_category_name(name: str) -> str:
    """Normalize category names across legacy and current benchmark payloads."""
    return CATEGORY_ALIASES.get(name, name)


def convert_to_ns(value: Any, unit: str) -> Optional[int]:
    """Convert a numeric benchmark value in the given unit to nanoseconds."""
    try:
        numeric = float(value)
    except (TypeError, ValueError):
        return None

    multiplier = UNIT_MULTIPLIERS.get(unit)
    if multiplier is None:
        return None

    return int(round(numeric * multiplier))


def get_threshold_for_benchmark(config: dict, category: str, bench_name: str) -> dict:
    """Get applicable thresholds for a specific benchmark."""
    # Start with defaults
    thresholds = config.get('defaults', {}).copy()

    # Apply category-specific overrides
    if category in config.get('categories', {}):
        thresholds.update(config['categories'][category])

    # Apply critical path overrides
    full_name = f"{category}/{bench_name}"
    for critical in config.get('critical_path', []):
        if critical['name'] == full_name:
            thresholds.update({
                'warn_threshold_pct': critical.get('warn_threshold_pct', thresholds.get('warn_threshold_pct')),
                'regression_threshold_pct': critical.get('regression_threshold_pct', thresholds.get('regression_threshold_pct')),
                'critical_threshold_pct': critical.get('critical_threshold_pct', thresholds.get('critical_threshold_pct')),
            })
            break

    return thresholds


def is_exempted(config: dict, category: str, bench_name: str) -> bool:
    """Check if benchmark is exempted from alerting."""
    full_name = f"{category}/{bench_name}"
    for pattern in config.get('exemptions', []):
        if pattern.endswith('/*') and full_name.startswith(pattern[:-2]):
            return True
        if pattern == full_name:
            return True
    return False


def classify_change(pct_change: float, thresholds: dict) -> Tuple[str, str, str]:
    """Classify performance change and return (status, color, emoji)."""
    warn_threshold = thresholds.get('warn_threshold_pct', 10)
    regression_threshold = thresholds.get('regression_threshold_pct', 20)
    critical_threshold = thresholds.get('critical_threshold_pct', 50)
    improvement_threshold = thresholds.get('improvement_threshold_pct', 10)

    if pct_change > critical_threshold:
        return ("CRITICAL", RED, "🔴")
    elif pct_change > regression_threshold:
        return ("REGRESSION", RED, "⚠️")
    elif pct_change > warn_threshold:
        return ("WARNING", YELLOW, "⚡")
    elif pct_change < -improvement_threshold:
        return ("IMPROVED", GREEN, "✅")
    else:
        return ("OK", NC, "✓")


def format_duration(ns: int) -> str:
    """Format nanoseconds as human-readable string."""
    if ns < 1000:
        return f"{ns}ns"
    elif ns < 1_000_000:
        return f"{ns / 1000:.1f}µs"
    elif ns < 1_000_000_000:
        return f"{ns / 1_000_000:.1f}ms"
    else:
        return f"{ns / 1_000_000_000:.2f}s"


def extract_mean_ns(bench_data: dict) -> Optional[int]:
    """Extract mean nanoseconds from benchmark data."""
    if "mean_ns" in bench_data:
        return convert_to_ns(bench_data["mean_ns"], "ns")
    for key, unit in (("mean_us", "us"), ("mean_ms", "ms"), ("mean_s", "s")):
        if key in bench_data:
            return convert_to_ns(bench_data[key], unit)
    if "mean" in bench_data:
        mean = bench_data["mean"]
        if isinstance(mean, dict):
            if "nanoseconds" in mean:
                return convert_to_ns(mean["nanoseconds"], "ns")
            if "microseconds" in mean:
                return convert_to_ns(mean["microseconds"], "us")
            if "milliseconds" in mean:
                return convert_to_ns(mean["milliseconds"], "ms")
            if "seconds" in mean:
                return convert_to_ns(mean["seconds"], "s")
            if "point_estimate" in mean:
                return convert_to_ns(mean["point_estimate"], "ns")
    return None


def extract_benchmarks(data: dict) -> Dict[str, Dict[str, Any]]:
    """Extract benchmark data from results."""
    results = {}

    def looks_like_benchmark(payload: Any) -> bool:
        return isinstance(payload, dict) and any(
            key in payload
            for key in ("mean", "mean_ns", "mean_us", "mean_ms", "mean_s")
        )

    if "results" in data:
        for category, benchmarks in data["results"].items():
            if isinstance(benchmarks, dict):
                normalized_category = normalize_category_name(category)
                category_results = results.setdefault(normalized_category, {})
                for name, values in benchmarks.items():
                    if name.startswith("_"):
                        continue
                    if isinstance(values, dict):
                        category_results[name] = values

    elif "benchmarks" in data:
        benchmarks = data["benchmarks"]
        for key, value in benchmarks.items():
            if isinstance(value, dict):
                if any(looks_like_benchmark(v) for v in value.values()):
                    normalized_category = normalize_category_name(key)
                    category_results = results.setdefault(normalized_category, {})
                    for name, bench_data in value.items():
                        if looks_like_benchmark(bench_data):
                            category_results[name] = bench_data
                elif looks_like_benchmark(value):
                    if "other" not in results:
                        results["other"] = {}
                    results["other"][key] = value

    return results


def generate_alert_terminal(baseline_file: Path, current_file: Path, config: dict) -> Tuple[dict, int]:
    """Generate terminal alert output and return (summary, exit_code)."""
    baseline_data = load_json(baseline_file)
    current_data = load_json(current_file)

    baseline_version = baseline_data.get("metadata", {}).get("version") or baseline_data.get("version", "unknown")
    baseline_sha = baseline_data.get("metadata", {}).get("git_sha") or baseline_data.get("git_sha", "unknown")
    current_sha = current_data.get("metadata", {}).get("git_sha") or current_data.get("git_sha", "unknown")

    baseline_benchmarks = extract_benchmarks(baseline_data)
    current_benchmarks = extract_benchmarks(current_data)

    summary = {
        'critical': [],
        'regressions': [],
        'warnings': [],
        'improvements': [],
        'unchanged': 0,
    }

    print(f"{BLUE}========================================")
    print("Performance Regression Alert")
    print(f"========================================{NC}")
    print()
    print(f"Baseline: v{baseline_version} ({baseline_sha[:8]})")
    print(f"Current:  {current_sha[:8]}")
    print()

    has_alerts = False

    for category in sorted(set(baseline_benchmarks.keys()) | set(current_benchmarks.keys())):
        if category not in baseline_benchmarks or category not in current_benchmarks:
            continue

        base_cat = baseline_benchmarks[category]
        curr_cat = current_benchmarks[category]

        category_alerts = []

        for bench_name in sorted(base_cat.keys()):
            if is_exempted(config, category, bench_name):
                continue

            base_bench = base_cat.get(bench_name)
            curr_bench = curr_cat.get(bench_name)

            if not base_bench or not curr_bench:
                continue

            base_ns = extract_mean_ns(base_bench)
            curr_ns = extract_mean_ns(curr_bench)

            if base_ns is None or curr_ns is None:
                continue

            pct_change = ((curr_ns - base_ns) * 100.0) / base_ns
            thresholds = get_threshold_for_benchmark(config, category, bench_name)
            status, color, emoji = classify_change(pct_change, thresholds)

            if status in ["CRITICAL", "REGRESSION", "WARNING", "IMPROVED"]:
                base_display = format_duration(base_ns)
                curr_display = format_duration(curr_ns)
                sign = "+" if pct_change >= 0 else ""

                alert = {
                    'category': category,
                    'name': bench_name,
                    'base': base_display,
                    'current': curr_display,
                    'pct_change': pct_change,
                    'status': status,
                    'color': color,
                    'emoji': emoji,
                }

                if status == "CRITICAL":
                    summary['critical'].append(alert)
                elif status == "REGRESSION":
                    summary['regressions'].append(alert)
                elif status == "WARNING":
                    summary['warnings'].append(alert)
                elif status == "IMPROVED":
                    summary['improvements'].append(alert)

                category_alerts.append(alert)
            else:
                summary['unchanged'] += 1

        if category_alerts:
            has_alerts = True
            print(f"{BLUE}{category}:{NC}")
            for alert in category_alerts:
                print(f"  {alert['emoji']} {alert['name']:30s}  {alert['base']:>10s} -> {alert['current']:<10s}  "
                      f"({'+' if alert['pct_change'] >= 0 else ''}{alert['pct_change']:.1f}%)  "
                      f"[{alert['color']}{alert['status']}{NC}]")
            print()

    if not has_alerts:
        print(f"{GREEN}No performance alerts detected.{NC}")
        print()

    # Summary
    print(f"{BLUE}========================================")
    print("Summary")
    print(f"========================================{NC}")
    print()
    print(f"  Critical:     {RED}{len(summary['critical'])}{NC}")
    print(f"  Regressions:  {RED}{len(summary['regressions'])}{NC}")
    print(f"  Warnings:     {YELLOW}{len(summary['warnings'])}{NC}")
    print(f"  Improvements: {GREEN}{len(summary['improvements'])}{NC}")
    print(f"  Unchanged:    {summary['unchanged']}")
    print()

    # Determine exit code
    exit_code = 0
    fail_on_critical = config.get('alerting', {}).get('fail_on_critical', False)
    if len(summary['critical']) > 0:
        if fail_on_critical:
            print(f"{RED}CRITICAL: Performance regression exceeds critical threshold.{NC}")
            exit_code = 1
        else:
            print(f"{YELLOW}WARNING: Critical performance regressions detected (not gated).{NC}")
    elif len(summary['regressions']) > 0:
        print(f"{YELLOW}WARNING: Performance regressions detected.{NC}")
    else:
        print(f"{GREEN}PASS: No significant performance regressions.{NC}")

    return summary, exit_code


def generate_alert_markdown(baseline_file: Path, current_file: Path, config: dict) -> str:
    """Generate markdown alert for GitHub PR comments."""
    baseline_data = load_json(baseline_file)
    current_data = load_json(current_file)

    baseline_version = baseline_data.get("metadata", {}).get("version") or baseline_data.get("version", "unknown")
    baseline_sha = baseline_data.get("metadata", {}).get("git_sha") or baseline_data.get("git_sha", "unknown")
    current_sha = current_data.get("metadata", {}).get("git_sha") or current_data.get("git_sha", "unknown")

    baseline_benchmarks = extract_benchmarks(baseline_data)
    current_benchmarks = extract_benchmarks(current_data)

    alerts = {
        'critical': [],
        'regressions': [],
        'warnings': [],
        'improvements': [],
    }

    for category in sorted(set(baseline_benchmarks.keys()) | set(current_benchmarks.keys())):
        if category not in baseline_benchmarks or category not in current_benchmarks:
            continue

        base_cat = baseline_benchmarks[category]
        curr_cat = current_benchmarks[category]

        for bench_name in sorted(base_cat.keys()):
            if is_exempted(config, category, bench_name):
                continue

            base_bench = base_cat.get(bench_name)
            curr_bench = curr_cat.get(bench_name)

            if not base_bench or not curr_bench:
                continue

            base_ns = extract_mean_ns(base_bench)
            curr_ns = extract_mean_ns(curr_bench)

            if base_ns is None or curr_ns is None:
                continue

            pct_change = ((curr_ns - base_ns) * 100.0) / base_ns
            thresholds = get_threshold_for_benchmark(config, category, bench_name)
            status, _, emoji = classify_change(pct_change, thresholds)

            if status in ["CRITICAL", "REGRESSION", "WARNING", "IMPROVED"]:
                base_display = format_duration(base_ns)
                curr_display = format_duration(curr_ns)

                alert = {
                    'category': category,
                    'name': bench_name,
                    'base': base_display,
                    'current': curr_display,
                    'pct_change': pct_change,
                    'status': status,
                    'emoji': emoji,
                }

                if status == "CRITICAL":
                    alerts['critical'].append(alert)
                elif status == "REGRESSION":
                    alerts['regressions'].append(alert)
                elif status == "WARNING":
                    alerts['warnings'].append(alert)
                elif status == "IMPROVED":
                    alerts['improvements'].append(alert)

    # Build markdown
    md = []
    md.append("## Performance Benchmark Results")
    md.append("")
    md.append(f"**Baseline:** v{baseline_version} ({baseline_sha[:8]})")
    md.append(f"**Current:**  {current_sha[:8]}")
    md.append("")

    if alerts['critical']:
        md.append("### 🔴 Critical Regressions")
        md.append("")
        md.append("| Benchmark | Baseline | Current | Change | Status |")
        md.append("|-----------|----------|---------|--------|--------|")
        for alert in alerts['critical']:
            md.append(f"| `{alert['category']}/{alert['name']}` | {alert['base']} | {alert['current']} | "
                     f"{'+' if alert['pct_change'] >= 0 else ''}{alert['pct_change']:.1f}% | {alert['emoji']} {alert['status']} |")
        md.append("")

    if alerts['regressions']:
        md.append("### ⚠️ Performance Regressions")
        md.append("")
        md.append("| Benchmark | Baseline | Current | Change | Status |")
        md.append("|-----------|----------|---------|--------|--------|")
        for alert in alerts['regressions']:
            md.append(f"| `{alert['category']}/{alert['name']}` | {alert['base']} | {alert['current']} | "
                     f"{'+' if alert['pct_change'] >= 0 else ''}{alert['pct_change']:.1f}% | {alert['emoji']} {alert['status']} |")
        md.append("")

    if alerts['warnings']:
        md.append("### ⚡ Performance Warnings")
        md.append("")
        md.append("| Benchmark | Baseline | Current | Change |")
        md.append("|-----------|----------|---------|--------|")
        for alert in alerts['warnings']:
            md.append(f"| `{alert['category']}/{alert['name']}` | {alert['base']} | {alert['current']} | "
                     f"{'+' if alert['pct_change'] >= 0 else ''}{alert['pct_change']:.1f}% |")
        md.append("")

    if alerts['improvements']:
        md.append("### ✅ Performance Improvements")
        md.append("")
        md.append("| Benchmark | Baseline | Current | Change |")
        md.append("|-----------|----------|---------|--------|")
        for alert in alerts['improvements']:
            md.append(f"| `{alert['category']}/{alert['name']}` | {alert['base']} | {alert['current']} | "
                     f"{alert['pct_change']:.1f}% |")
        md.append("")

    if not any([alerts['critical'], alerts['regressions'], alerts['warnings'], alerts['improvements']]):
        md.append("✅ No significant performance changes detected.")
        md.append("")

    md.append("---")
    md.append("")
    md.append(f"📊 **Summary:** {len(alerts['critical'])} critical, {len(alerts['regressions'])} regressions, "
             f"{len(alerts['warnings'])} warnings, {len(alerts['improvements'])} improvements")
    md.append("")
    md.append("<sub>Thresholds configured in `.ci/benchmark-thresholds.yaml`</sub>")

    return "\n".join(md)


def _semver_key(path: Path) -> Tuple[int, ...]:
    """Extract (major, minor, patch) from a filename like v0.9.1.json."""
    m = re.search(r'v(\d+)\.(\d+)\.(\d+)', path.stem)
    if m:
        return (int(m.group(1)), int(m.group(2)), int(m.group(3)))
    return (0, 0, 0)


def main():
    parser = argparse.ArgumentParser(description="Generate performance regression alerts")
    parser.add_argument("baseline", nargs="?", help="Baseline JSON file")
    parser.add_argument("current", nargs="?", help="Current results JSON file")
    parser.add_argument("--config", "-c", default=".ci/benchmark-thresholds.yaml",
                       help="Threshold configuration file (default: .ci/benchmark-thresholds.yaml)")
    parser.add_argument("--format", "-f", choices=["terminal", "markdown"], default="terminal",
                       help="Output format (default: terminal)")
    parser.add_argument("--check", action="store_true",
                       help="Exit non-zero if critical regression detected")
    args = parser.parse_args()

    # Determine paths
    repo_root = Path(__file__).parent.parent.parent
    config_path = repo_root / args.config

    if not config_path.exists():
        print(f"Error: Config file not found: {config_path}", file=sys.stderr)
        sys.exit(1)

    config = load_config(config_path)

    # Find baseline
    if args.baseline:
        baseline_file = Path(args.baseline)
    else:
        baselines_dir = repo_root / "benchmarks" / "baselines"
        baseline_files = sorted(baselines_dir.glob("*.json"), key=_semver_key, reverse=True)
        baseline_file = baseline_files[0] if baseline_files else None

    if baseline_file is None or not baseline_file.exists():
        print("Error: No baseline file found", file=sys.stderr)
        print("Run 'just bench-baseline' to create one.", file=sys.stderr)
        sys.exit(1)

    # Find current results
    current_file = Path(args.current) if args.current else repo_root / "benchmarks" / "results" / "latest.json"

    if not current_file.exists():
        print(f"Error: Current results file not found: {current_file}", file=sys.stderr)
        print("Run 'just bench' first to generate results.", file=sys.stderr)
        sys.exit(1)

    # Generate alert
    if args.format == "markdown":
        md = generate_alert_markdown(baseline_file, current_file, config)
        print(md)
        sys.exit(0)
    else:
        summary, exit_code = generate_alert_terminal(baseline_file, current_file, config)
        if args.check:
            sys.exit(exit_code)
        else:
            sys.exit(0)


if __name__ == "__main__":
    main()

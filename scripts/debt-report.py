#!/usr/bin/env python3
"""
Debt Ledger Report Generator

Reads .ci/debt-ledger.yaml and generates a report on current technical debt,
flaky tests, and known issues. Can be used for:
  - Console output (just debt-report)
  - CI gates (just debt-check)
  - Receipt integration (json output)

Usage:
    python3 scripts/debt-report.py              # Console report
    python3 scripts/debt-report.py --check      # CI gate (exit 1 if over budget)
    python3 scripts/debt-report.py --json       # JSON output for receipts
    python3 scripts/debt-report.py --expired    # Show expired quarantines only
"""

import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

# Try to import yaml, fall back to basic parsing if not available
try:
    import yaml
    HAS_YAML = True
except ImportError:
    HAS_YAML = False


def parse_yaml_simple(content: str) -> dict[str, Any]:
    """Simple YAML parser for basic structures (fallback when PyYAML unavailable)."""
    # This is a very basic parser that handles the debt ledger structure
    # For production use, PyYAML should be installed
    result: dict[str, Any] = {
        "schema_version": 1,
        "budgets": {
            "max_quarantined_tests": 10,
            "max_known_issues": 20,
            "max_technical_debt": 30,
            "warning_threshold_percent": 80,
            "critical_threshold_percent": 95,
        },
        "flaky_tests": [],
        "known_issues": [],
        "technical_debt": [],
    }

    # Extract budget values with regex-like parsing
    import re

    for line in content.split("\n"):
        line = line.strip()
        if line.startswith("max_quarantined_tests:"):
            match = re.search(r":\s*(\d+)", line)
            if match:
                result["budgets"]["max_quarantined_tests"] = int(match.group(1))
        elif line.startswith("max_known_issues:"):
            match = re.search(r":\s*(\d+)", line)
            if match:
                result["budgets"]["max_known_issues"] = int(match.group(1))
        elif line.startswith("max_technical_debt:"):
            match = re.search(r":\s*(\d+)", line)
            if match:
                result["budgets"]["max_technical_debt"] = int(match.group(1))

    # For the list sections, we need more complex parsing
    # This fallback is intentionally simple - use PyYAML for full support

    return result


def load_ledger(ledger_path: Path) -> dict[str, Any]:
    """Load and parse the debt ledger YAML file."""
    if not ledger_path.exists():
        return {
            "schema_version": 1,
            "budgets": {
                "max_quarantined_tests": 10,
                "max_known_issues": 20,
                "max_technical_debt": 30,
                "warning_threshold_percent": 80,
                "critical_threshold_percent": 95,
            },
            "flaky_tests": [],
            "known_issues": [],
            "technical_debt": [],
        }

    content = ledger_path.read_text()

    if HAS_YAML:
        return yaml.safe_load(content)
    else:
        print("Warning: PyYAML not installed, using basic parser", file=sys.stderr)
        return parse_yaml_simple(content)


def calculate_expiry(item: dict[str, Any]) -> datetime | None:
    """Calculate expiry date for a quarantined item."""
    if "expires" in item:
        try:
            return datetime.strptime(item["expires"], "%Y-%m-%d")
        except (ValueError, TypeError):
            pass

    if "added" in item and "quarantine_days" in item:
        try:
            added = datetime.strptime(item["added"], "%Y-%m-%d")
            return added + timedelta(days=item["quarantine_days"])
        except (ValueError, TypeError):
            pass

    return None


def is_expired(item: dict[str, Any], now: datetime) -> bool:
    """Check if a quarantined item has expired."""
    expiry = calculate_expiry(item)
    return expiry is not None and expiry < now


def days_until_expiry(item: dict[str, Any], now: datetime) -> int | None:
    """Get days until expiry (negative if expired)."""
    expiry = calculate_expiry(item)
    if expiry is None:
        return None
    return (expiry - now).days


def count_by_status(items: list[dict[str, Any]], status: str) -> int:
    """Count items with a specific status."""
    return sum(1 for item in items if item.get("status") == status)


def count_by_priority(items: list[dict[str, Any]], priority: str) -> int:
    """Count items with a specific priority."""
    return sum(1 for item in items if item.get("priority") == priority)


def generate_report(ledger: dict[str, Any], now: datetime) -> dict[str, Any]:
    """Generate a comprehensive debt report."""
    budgets = ledger.get("budgets", {})
    flaky_tests = ledger.get("flaky_tests", []) or []
    known_issues = ledger.get("known_issues", []) or []
    technical_debt = ledger.get("technical_debt", []) or []

    # Count items
    quarantined_count = len(flaky_tests)
    known_issues_count = len(known_issues)
    tech_debt_count = len(technical_debt)

    # Budget limits
    max_quarantined = budgets.get("max_quarantined_tests", 10)
    max_issues = budgets.get("max_known_issues", 20)
    max_debt = budgets.get("max_technical_debt", 30)
    warning_pct = budgets.get("warning_threshold_percent", 80)
    critical_pct = budgets.get("critical_threshold_percent", 95)

    # Calculate percentages
    quarantine_pct = (quarantined_count / max_quarantined * 100) if max_quarantined > 0 else 0
    issues_pct = (known_issues_count / max_issues * 100) if max_issues > 0 else 0
    debt_pct = (tech_debt_count / max_debt * 100) if max_debt > 0 else 0

    # Find expired quarantines
    expired_quarantines = [item for item in flaky_tests if is_expired(item, now)]
    expiring_soon = [
        item
        for item in flaky_tests
        if not is_expired(item, now)
        and days_until_expiry(item, now) is not None
        and 0 <= (days_until_expiry(item, now) or 0) <= 7
    ]

    # Determine status
    def get_status(pct: float) -> str:
        if pct >= critical_pct:
            return "critical"
        elif pct >= warning_pct:
            return "warning"
        else:
            return "ok"

    overall_status = "ok"
    if any(get_status(p) == "critical" for p in [quarantine_pct, issues_pct, debt_pct]):
        overall_status = "critical"
    elif any(get_status(p) == "warning" for p in [quarantine_pct, issues_pct, debt_pct]):
        overall_status = "warning"

    # Add expired quarantine check
    if expired_quarantines:
        overall_status = "critical"

    return {
        "timestamp": now.isoformat() + "Z",
        "schema_version": ledger.get("schema_version", 1),
        "summary": {
            "overall_status": overall_status,
            "quarantined_tests": {
                "count": quarantined_count,
                "budget": max_quarantined,
                "percent": round(quarantine_pct, 1),
                "status": get_status(quarantine_pct),
                "expired": len(expired_quarantines),
                "expiring_soon": len(expiring_soon),
            },
            "known_issues": {
                "count": known_issues_count,
                "budget": max_issues,
                "percent": round(issues_pct, 1),
                "status": get_status(issues_pct),
                "by_status": {
                    "accepted": count_by_status(known_issues, "accepted"),
                    "deferred": count_by_status(known_issues, "deferred"),
                    "monitoring": count_by_status(known_issues, "monitoring"),
                    "wontfix": count_by_status(known_issues, "wontfix"),
                },
            },
            "technical_debt": {
                "count": tech_debt_count,
                "budget": max_debt,
                "percent": round(debt_pct, 1),
                "status": get_status(debt_pct),
                "by_priority": {
                    "critical": count_by_priority(technical_debt, "critical"),
                    "high": count_by_priority(technical_debt, "high"),
                    "medium": count_by_priority(technical_debt, "medium"),
                    "low": count_by_priority(technical_debt, "low"),
                },
            },
        },
        "alerts": [],
        "details": {
            "expired_quarantines": [
                {
                    "name": item.get("name"),
                    "issue": item.get("issue"),
                    "expired": calculate_expiry(item).strftime("%Y-%m-%d") if calculate_expiry(item) else None,
                    "days_overdue": -1 * (days_until_expiry(item, now) or 0),
                }
                for item in expired_quarantines
            ],
            "expiring_soon": [
                {
                    "name": item.get("name"),
                    "issue": item.get("issue"),
                    "expires": calculate_expiry(item).strftime("%Y-%m-%d") if calculate_expiry(item) else None,
                    "days_remaining": days_until_expiry(item, now),
                }
                for item in expiring_soon
            ],
            "critical_debt": [
                {
                    "area": item.get("area"),
                    "description": item.get("description"),
                    "issue": item.get("issue"),
                }
                for item in technical_debt
                if item.get("priority") == "critical"
            ],
        },
    }


def format_console_report(report: dict[str, Any]) -> str:
    """Format report for console output."""
    lines = []
    summary = report["summary"]

    # Status colors (ANSI)
    colors = {
        "ok": "\033[32m",      # Green
        "warning": "\033[33m", # Yellow
        "critical": "\033[31m", # Red
        "reset": "\033[0m",
    }

    def status_color(status: str) -> str:
        return colors.get(status, "") + status.upper() + colors["reset"]

    lines.append("=" * 60)
    lines.append("           Technical Debt Report")
    lines.append("=" * 60)
    lines.append(f"Generated: {report['timestamp']}")
    lines.append(f"Overall Status: {status_color(summary['overall_status'])}")
    lines.append("")

    # Quarantined Tests
    q = summary["quarantined_tests"]
    lines.append(f"Quarantined Tests: {q['count']}/{q['budget']} ({q['percent']}%) [{status_color(q['status'])}]")
    if q["expired"] > 0:
        lines.append(f"  {colors['critical']}EXPIRED: {q['expired']} quarantine(s) need resolution!{colors['reset']}")
    if q["expiring_soon"] > 0:
        lines.append(f"  {colors['warning']}Expiring soon: {q['expiring_soon']} within 7 days{colors['reset']}")

    # Known Issues
    k = summary["known_issues"]
    lines.append(f"Known Issues: {k['count']}/{k['budget']} ({k['percent']}%) [{status_color(k['status'])}]")
    by_status = k["by_status"]
    if any(by_status.values()):
        status_parts = [f"{s}: {c}" for s, c in by_status.items() if c > 0]
        lines.append(f"  {', '.join(status_parts)}")

    # Technical Debt
    t = summary["technical_debt"]
    lines.append(f"Technical Debt: {t['count']}/{t['budget']} ({t['percent']}%) [{status_color(t['status'])}]")
    by_priority = t["by_priority"]
    if any(by_priority.values()):
        priority_parts = [f"{p}: {c}" for p, c in by_priority.items() if c > 0]
        lines.append(f"  {', '.join(priority_parts)}")

    # Details
    details = report["details"]

    if details["expired_quarantines"]:
        lines.append("")
        lines.append(f"{colors['critical']}Expired Quarantines (action required):{colors['reset']}")
        for item in details["expired_quarantines"]:
            issue = f" ({item['issue']})" if item.get("issue") else ""
            lines.append(f"  - {item['name']}{issue}: {item['days_overdue']} days overdue")

    if details["expiring_soon"]:
        lines.append("")
        lines.append(f"{colors['warning']}Expiring Soon:{colors['reset']}")
        for item in details["expiring_soon"]:
            issue = f" ({item['issue']})" if item.get("issue") else ""
            lines.append(f"  - {item['name']}{issue}: {item['days_remaining']} days remaining")

    if details["critical_debt"]:
        lines.append("")
        lines.append(f"{colors['critical']}Critical Technical Debt:{colors['reset']}")
        for item in details["critical_debt"]:
            issue = f" ({item['issue']})" if item.get("issue") else ""
            lines.append(f"  - [{item['area']}] {item['description']}{issue}")

    lines.append("")
    lines.append("=" * 60)
    lines.append("Run `just debt-check` to verify debt budget compliance")
    lines.append("Edit `.ci/debt-ledger.yaml` to add/remove tracked items")
    lines.append("=" * 60)

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Generate technical debt report from debt ledger"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="CI gate mode: exit 1 if over budget or expired quarantines",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON format for receipt integration",
    )
    parser.add_argument(
        "--expired",
        action="store_true",
        help="Show only expired quarantines",
    )
    parser.add_argument(
        "--ledger",
        type=Path,
        default=None,
        help="Path to debt ledger (default: .ci/debt-ledger.yaml)",
    )

    args = parser.parse_args()

    # Find repo root and ledger
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent
    ledger_path = args.ledger or (repo_root / ".ci" / "debt-ledger.yaml")

    # Load ledger
    ledger = load_ledger(ledger_path)

    # Generate report
    now = datetime.now(timezone.utc).replace(tzinfo=None)
    report = generate_report(ledger, now)

    # Handle expired-only mode
    if args.expired:
        expired = report["details"]["expired_quarantines"]
        if args.json:
            print(json.dumps(expired, indent=2))
        else:
            if expired:
                print("Expired Quarantines:")
                for item in expired:
                    print(f"  - {item['name']}: {item['days_overdue']} days overdue")
                sys.exit(1)
            else:
                print("No expired quarantines")
        sys.exit(0)

    # Output format
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print(format_console_report(report))

    # Check mode - exit 1 if over budget or expired
    if args.check:
        summary = report["summary"]
        failures = []

        # Check expired quarantines
        expired_count = summary["quarantined_tests"]["expired"]
        if expired_count > 0:
            failures.append(f"{expired_count} expired quarantine(s)")

        # Check budgets at critical level
        if summary["quarantined_tests"]["status"] == "critical":
            failures.append("quarantined tests at critical level")
        if summary["known_issues"]["status"] == "critical":
            failures.append("known issues at critical level")
        if summary["technical_debt"]["status"] == "critical":
            failures.append("technical debt at critical level")

        if failures:
            print(f"\nDebt check FAILED: {', '.join(failures)}", file=sys.stderr)
            sys.exit(1)
        else:
            print("\nDebt check PASSED")
            sys.exit(0)


if __name__ == "__main__":
    main()

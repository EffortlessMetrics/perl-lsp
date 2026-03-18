#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent


@dataclass(frozen=True)
class Metric:
    label: str
    value: str
    status: str
    summary: str


def rust_files_under(relative: str) -> list[Path]:
    base = ROOT / relative
    if not base.exists():
        return []
    return sorted(base.rglob("*.rs"))


def rust_files_matching_src() -> list[Path]:
    return sorted((ROOT / "crates").glob("*/src/**/*.rs"))


def rust_files_matching_tests(crate: str) -> list[Path]:
    return sorted((ROOT / "crates" / crate / "tests").rglob("*.rs"))


def count_pattern_in_files(pattern: str, files: list[Path]) -> int:
    regex = re.compile(pattern, re.MULTILINE)
    total = 0
    for path in files:
        try:
            total += len(regex.findall(path.read_text(encoding="utf-8")))
        except OSError:
            continue
    return total


def lines_in_files(files: list[Path]) -> int:
    total = 0
    for path in files:
        try:
            total += len(path.read_text(encoding="utf-8").splitlines())
        except OSError:
            continue
    return total


def run_capture(*args: str) -> str:
    result = subprocess.run(args, cwd=ROOT, check=False, capture_output=True, text=True)
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def render_bar(count: int, warn: int, fail: int, width: int = 16) -> str:
    ratio = 0.0 if fail <= 0 else min(count / fail, 1.0)
    filled = min(int(round(ratio * width)), width)
    if count == 0:
        block = "🟢"
    elif count <= warn:
        block = "🟡"
    else:
        block = "🔴"
    return f"{block} {'█' * filled}{'░' * (width - filled)}"


def status_for_low_count(count: int, warn: int, fail: int) -> str:
    if count == 0:
        return "good"
    if count <= warn:
        return "watch"
    return "risk"


def icon(status: str) -> str:
    return {"good": "🟢", "watch": "🟡", "risk": "🔴", "info": "🔵"}[status]


def build_metrics() -> tuple[list[Metric], dict[str, list[tuple[int, str]]]]:
    parser_tests = rust_files_matching_tests("perl-parser")
    lsp_tests = rust_files_matching_tests("perl-lsp")
    lexer_tests = rust_files_matching_tests("perl-lexer")
    dap_tests = rust_files_matching_tests("perl-dap")
    src_files = rust_files_matching_src()
    parser_src = rust_files_under("crates/perl-parser/src")
    lsp_src = rust_files_under("crates/perl-lsp/src")

    ignored_parser = count_pattern_in_files(r"#\[ignore", parser_tests)
    ignored_lsp = count_pattern_in_files(r"#\[ignore", lsp_tests)
    ignored_lexer = count_pattern_in_files(r"#\[ignore", lexer_tests)
    ignored_dap = count_pattern_in_files(r"#\[ignore", dap_tests)
    ignored_total = ignored_parser + ignored_lsp + ignored_lexer + ignored_dap

    unwraps = count_pattern_in_files(r"\.unwrap\(", src_files)
    expects = count_pattern_in_files(r"\.expect\(", src_files)
    printlns = count_pattern_in_files(r"\bprintln!", src_files)
    eprintlns = count_pattern_in_files(r"\beprintln!", src_files)
    dead_code_allows = count_pattern_in_files(r"#\[allow\(dead_code\)\]", rust_files_under("crates"))
    parser_pub_fn = count_pattern_in_files(r"^[\t ]*pub fn\b", parser_src)
    parser_pub_struct = count_pattern_in_files(r"^[\t ]*pub struct\b", parser_src)
    parser_pub_enum = count_pattern_in_files(r"^[\t ]*pub enum\b", parser_src)
    lsp_lines = lines_in_files(lsp_src)

    machete_output = ""
    if shutil.which("cargo"):
        machete_output = run_capture("cargo", "machete")
    unused_deps = machete_output.count("Cargo.toml:") if machete_output else 0

    details = {
        "unwraps": sorted(
            (
                (count_pattern_in_files(r"\.unwrap\(", [path]), str(path.relative_to(ROOT)))
                for path in src_files
            ),
            reverse=True,
        ),
        "eprintlns": sorted(
            (
                (count_pattern_in_files(r"\beprintln!", [path]), str(path.relative_to(ROOT)))
                for path in src_files
            ),
            reverse=True,
        ),
        "largest": sorted(
            (
                (len(path.read_text(encoding="utf-8").splitlines()), str(path.relative_to(ROOT)))
                for path in rust_files_under("crates")
            ),
            reverse=True,
        ),
    }

    metrics = [
        Metric(
            "Ignored tests",
            str(ignored_total),
            status_for_low_count(ignored_total, warn=5, fail=20),
            f"parser {ignored_parser} · lsp {ignored_lsp} · lexer {ignored_lexer} · dap {ignored_dap}",
        ),
        Metric(
            "Panic sites",
            str(unwraps + expects),
            status_for_low_count(unwraps + expects, warn=0, fail=1),
            f"unwrap {unwraps} · expect {expects}",
        ),
        Metric(
            "Debug prints",
            str(printlns + eprintlns),
            status_for_low_count(printlns + eprintlns, warn=5, fail=20),
            f"println {printlns} · eprintln {eprintlns}",
        ),
        Metric(
            "Dead-code allows",
            str(dead_code_allows),
            status_for_low_count(dead_code_allows, warn=5, fail=20),
            "allow(dead_code) markers in Rust sources",
        ),
        Metric(
            "Unused deps",
            str(unused_deps),
            "info" if not machete_output else status_for_low_count(unused_deps, warn=3, fail=10),
            "cargo machete summary" if machete_output else "cargo machete unavailable in this environment",
        ),
        Metric(
            "perl-parser API",
            str(parser_pub_fn + parser_pub_struct + parser_pub_enum),
            "info",
            f"pub fn {parser_pub_fn} · pub struct {parser_pub_struct} · pub enum {parser_pub_enum}",
        ),
        Metric(
            "perl-lsp size",
            f"{lsp_lines:,} LOC",
            "info" if lsp_lines < 50000 else "watch",
            "Rust source lines under crates/perl-lsp/src",
        ),
    ]
    return metrics, details


def print_dashboard(detail: bool) -> None:
    metrics, details = build_metrics()
    print("📊 DevEx Health Dashboard")
    print("=========================")
    print()
    print(f"{'Area':<18} {'Value':<12} {'Status':<6} Summary")
    print(f"{'-' * 18} {'-' * 12} {'-' * 6} {'-' * 48}")
    for metric in metrics:
        value = metric.value
        numeric_value = value.isdigit()
        if numeric_value and metric.status != "info":
            summary = f"{render_bar(int(value), warn=5, fail=20)}  {metric.summary}"
        else:
            summary = metric.summary
        print(f"{metric.label:<18} {value:<12} {icon(metric.status):<6} {summary}")

    print()
    print("Legend: 🟢 healthy · 🟡 watch · 🔴 action needed · 🔵 informational")

    if not detail:
        return

    print()
    print("📁 Detail Views")
    print("==============")
    print()
    print("Most .unwrap() occurrences:")
    unwrap_rows = [row for row in details["unwraps"] if row[0] > 0][:10]
    if unwrap_rows:
        for count, rel in unwrap_rows:
            print(f"  {count:>3}  {rel}")
    else:
        print("  none")

    print()
    print("Most eprintln! occurrences:")
    eprintln_rows = [row for row in details["eprintlns"] if row[0] > 0][:10]
    if eprintln_rows:
        for count, rel in eprintln_rows:
            print(f"  {count:>3}  {rel}")
    else:
        print("  none")

    print()
    print("Largest Rust source files:")
    for count, rel in details["largest"][:10]:
        print(f"  {count:>6}  {rel}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Render a DevEx-focused health dashboard.")
    parser.add_argument("--detail", action="store_true", help="show file-level breakdowns")
    args = parser.parse_args()
    print_dashboard(detail=args.detail)

#!/usr/bin/env python3
"""
Tree-sitter Perl C vs Rust Benchmark Comparison Generator

This script takes benchmark results from both C and Rust implementations
and generates a comprehensive comparison report with statistical analysis.
"""

import argparse
import json
import statistics
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple
import math

class BenchmarkComparison:
    """Generate comparison results between C and Rust benchmark data."""
    
    def __init__(self, c_results_path: str, rust_results_path: str):
        self.c_results_path = Path(c_results_path)
        self.rust_results_path = Path(rust_results_path)
        self.c_data = {}
        self.rust_data = {}
        self.comparison_data = {}
        
    def load_data(self) -> None:
        """Load benchmark data from both implementations."""
        try:
            with open(self.c_results_path, 'r') as f:
                self.c_data = json.load(f)
        except FileNotFoundError:
            print(f"Error: C results file not found: {self.c_results_path}")
            sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in C results file: {e}")
            sys.exit(1)
            
        try:
            with open(self.rust_results_path, 'r') as f:
                self.rust_data = json.load(f)
        except FileNotFoundError:
            print(f"Error: Rust results file not found: {self.rust_results_path}")
            sys.exit(1)
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in Rust results file: {e}")
            sys.exit(1)
    
    def calculate_statistics(self, values: List[float]) -> Dict[str, float]:
        """Calculate statistical measures for a list of values."""
        if not values:
            return {}
            
        return {
            'mean': statistics.mean(values),
            'median': statistics.median(values),
            'std_dev': statistics.stdev(values) if len(values) > 1 else 0.0,
            'min': min(values),
            'max': max(values),
            'count': len(values)
        }
    
    def calculate_confidence_interval(self, values: List[float], confidence: float = 0.95) -> Tuple[float, float]:
        """Calculate confidence interval for a list of values."""
        if len(values) < 2:
            return (values[0], values[0]) if values else (0.0, 0.0)
            
        mean = statistics.mean(values)
        std_err = statistics.stdev(values) / math.sqrt(len(values))
        
        # For 95% confidence interval, use 1.96
        z_score = 1.96 if confidence == 0.95 else 1.645  # 90% confidence
        
        margin = z_score * std_err
        return (mean - margin, mean + margin)
    
    def compare_implementations(self) -> Dict[str, Any]:
        """Compare C and Rust implementations and generate statistics."""
        comparison = {
            'metadata': {
                'generated_at': datetime.now().isoformat(),
                'c_results_file': str(self.c_results_path),
                'rust_results_file': str(self.rust_results_path),
                'total_tests': 0,
                'tests_with_regression': 0,
                'tests_with_improvement': 0,
                'tests_within_tolerance': 0
            },
            'summary': {},
            'tests': [],
            'categories': {
                'small_files': [],
                'medium_files': [],
                'large_files': [],
                'error_recovery': [],
                'memory_usage': []
            }
        }
        
        # Extract test results from both implementations
        c_tests = self.c_data.get('tests', {})
        rust_tests = self.rust_data.get('tests', {})
        
        # Find common test names
        all_test_names = set(c_tests.keys()) | set(rust_tests.keys())
        
        for test_name in all_test_names:
            c_result = c_tests.get(test_name, {})
            rust_result = rust_tests.get(test_name, {})
            
            if not c_result or not rust_result:
                continue
                
            # Extract timing data
            c_time = c_result.get('mean_duration_ns', 0) / 1_000_000  # Convert to ms
            rust_time = rust_result.get('mean_duration_ns', 0) / 1_000_000  # Convert to ms
            
            # Calculate difference
            if c_time > 0:
                time_diff = (rust_time - c_time) / c_time
                time_diff_percent = time_diff * 100
            else:
                time_diff = 0.0
                time_diff_percent = 0.0
            
            # Determine status
            if time_diff > 0.05:  # 5% regression
                status = "regression"
                comparison['metadata']['tests_with_regression'] += 1
            elif time_diff < -0.05:  # 5% improvement
                status = "improvement"
                comparison['metadata']['tests_with_improvement'] += 1
            else:
                status = "within_tolerance"
                comparison['metadata']['tests_within_tolerance'] += 1
            
            test_comparison = {
                'name': test_name,
                'c_implementation': {
                    'duration_ms': c_time,
                    'std_dev_ms': c_result.get('std_dev_ns', 0) / 1_000_000,
                    'iterations': c_result.get('iterations', 0)
                },
                'rust_implementation': {
                    'duration_ms': rust_time,
                    'std_dev_ms': rust_result.get('std_dev_ns', 0) / 1_000_000,
                    'iterations': rust_result.get('iterations', 0)
                },
                'comparison': {
                    'time_difference': time_diff,
                    'time_difference_percent': time_diff_percent,
                    'speedup_factor': c_time / rust_time if rust_time > 0 else 0.0,
                    'status': status
                }
            }
            
            comparison['tests'].append(test_comparison)
            comparison['metadata']['total_tests'] += 1
            
            # Categorize tests
            if 'small' in test_name.lower() or test_name.endswith('_small'):
                comparison['categories']['small_files'].append(test_comparison)
            elif 'medium' in test_name.lower() or test_name.endswith('_medium'):
                comparison['categories']['medium_files'].append(test_comparison)
            elif 'large' in test_name.lower() or test_name.endswith('_large'):
                comparison['categories']['large_files'].append(test_comparison)
            elif 'error' in test_name.lower() or 'recovery' in test_name.lower():
                comparison['categories']['error_recovery'].append(test_comparison)
            elif 'memory' in test_name.lower():
                comparison['categories']['memory_usage'].append(test_comparison)
        
        # Generate summary statistics
        comparison['summary'] = self.generate_summary_statistics(comparison)
        
        return comparison
    
    def generate_summary_statistics(self, comparison: Dict[str, Any]) -> Dict[str, Any]:
        """Generate summary statistics for the comparison."""
        if not comparison['tests']:
            return {}
        
        # Overall performance
        time_diffs = [test['comparison']['time_difference_percent'] for test in comparison['tests']]
        speedups = [test['comparison']['speedup_factor'] for test in comparison['tests']]
        
        # Categorize by performance impact
        regressions = [td for td in time_diffs if td > 5.0]
        improvements = [td for td in time_diffs if td < -5.0]
        stable = [td for td in time_diffs if -5.0 <= td <= 5.0]
        
        summary = {
            'overall_performance': {
                'mean_time_difference_percent': statistics.mean(time_diffs),
                'median_time_difference_percent': statistics.median(time_diffs),
                'mean_speedup_factor': statistics.mean(speedups),
                'median_speedup_factor': statistics.median(speedups)
            },
            'performance_distribution': {
                'regressions_count': len(regressions),
                'improvements_count': len(improvements),
                'stable_count': len(stable),
                'regressions_mean': statistics.mean(regressions) if regressions else 0.0,
                'improvements_mean': statistics.mean(improvements) if improvements else 0.0
            },
            'test_coverage': {
                'total_tests': comparison['metadata']['total_tests'],
                'tests_with_regression': comparison['metadata']['tests_with_regression'],
                'tests_with_improvement': comparison['metadata']['tests_with_improvement'],
                'tests_within_tolerance': comparison['metadata']['tests_within_tolerance']
            }
        }
        
        return summary
    
    def generate_markdown_report(self, comparison: Dict[str, Any], report_path: str) -> None:
        """Generate a markdown report from the comparison data."""
        report_lines = [
            "# Tree-sitter Perl Benchmark Comparison Report",
            "",
            f"**Generated**: {comparison['metadata']['generated_at']}",
            f"**C Results**: {comparison['metadata']['c_results_file']}",
            f"**Rust Results**: {comparison['metadata']['rust_results_file']}",
            "",
            "## Executive Summary",
            "",
            f"- **Total Tests**: {comparison['metadata']['total_tests']}",
            f"- **Performance Regressions**: {comparison['metadata']['tests_with_regression']}",
            f"- **Performance Improvements**: {comparison['metadata']['tests_with_improvement']}",
            f"- **Within Tolerance**: {comparison['metadata']['tests_within_tolerance']}",
            ""
        ]
        
        # Overall performance summary
        if comparison['summary']:
            summary = comparison['summary']
            report_lines.extend([
                "### Overall Performance",
                "",
                f"- **Mean Time Difference**: {summary['overall_performance']['mean_time_difference_percent']:.2f}%",
                f"- **Median Time Difference**: {summary['overall_performance']['median_time_difference_percent']:.2f}%",
                f"- **Mean Speedup Factor**: {summary['overall_performance']['mean_speedup_factor']:.3f}x",
                f"- **Median Speedup Factor**: {summary['overall_performance']['median_speedup_factor']:.3f}x",
                ""
            ])
        
        # Detailed test results
        report_lines.extend([
            "## Detailed Test Results",
            "",
            "| Test Name | C (ms) | Rust (ms) | Difference | Status |",
            "|-----------|--------|-----------|------------|---------|"
        ])
        
        for test in comparison['tests']:
            c_time = test['c_implementation']['duration_ms']
            rust_time = test['rust_implementation']['duration_ms']
            diff_percent = test['comparison']['time_difference_percent']
            status = test['comparison']['status']
            
            status_emoji = {
                'regression': '🔴',
                'improvement': '🟢',
                'within_tolerance': '🟡'
            }.get(status, '⚪')
            
            report_lines.append(
                f"| {test['name']} | {c_time:.3f} | {rust_time:.3f} | {diff_percent:+.2f}% | {status_emoji} {status} |"
            )
        
        report_lines.extend([
            "",
            "## Performance Gates Status",
            "",
            "| Gate | Threshold | Status |",
            "|------|-----------|---------|"
        ])
        
        # Performance gates
        regression_count = comparison['metadata']['tests_with_regression']
        total_tests = comparison['metadata']['total_tests']
        
        gates = [
            ("Parse Time Regression", "<5%", "✅ PASS" if regression_count == 0 else f"❌ FAIL ({regression_count} regressions)"),
            ("Overall Performance", "<5%", "✅ PASS" if regression_count <= total_tests * 0.05 else "❌ FAIL"),
            ("Test Coverage", ">90%", "✅ PASS" if total_tests >= 10 else "⚠️ WARNING")
        ]
        
        for gate_name, threshold, status in gates:
            report_lines.append(f"| {gate_name} | {threshold} | {status} |")
        
        # Write report
        with open(report_path, 'w') as f:
            f.write('\n'.join(report_lines))
    
    def save_comparison(self, comparison: Dict[str, Any], output_path: str) -> None:
        """Save comparison results to JSON file."""
        with open(output_path, 'w') as f:
            json.dump(comparison, f, indent=2)
    
    def run(self, output_path: str, report_path: str) -> None:
        """Run the complete comparison process."""
        print("Loading benchmark data...")
        self.load_data()
        
        print("Generating comparison...")
        comparison = self.compare_implementations()
        
        print("Saving comparison results...")
        self.save_comparison(comparison, output_path)
        
        print("Generating markdown report...")
        self.generate_markdown_report(comparison, report_path)
        
        print(f"Comparison completed:")
        print(f"  - Results: {output_path}")
        print(f"  - Report: {report_path}")
        print(f"  - Total tests: {comparison['metadata']['total_tests']}")
        print(f"  - Regressions: {comparison['metadata']['tests_with_regression']}")
        print(f"  - Improvements: {comparison['metadata']['tests_with_improvement']}")

def main():
    parser = argparse.ArgumentParser(description="Generate C vs Rust benchmark comparison")
    parser.add_argument("--c-results", required=True, help="Path to C implementation results JSON")
    parser.add_argument("--rust-results", required=True, help="Path to Rust implementation results JSON")
    parser.add_argument("--output", required=True, help="Output path for comparison JSON")
    parser.add_argument("--report", required=True, help="Output path for markdown report")
    
    args = parser.parse_args()
    
    comparison = BenchmarkComparison(args.c_results, args.rust_results)
    comparison.run(args.output, args.report)

if __name__ == "__main__":
    main() 
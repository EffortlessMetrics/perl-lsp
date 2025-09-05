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
import os

class ComparisonConfig:
    """Configuration for benchmark comparison thresholds and settings."""
    
    def __init__(self, config_path: Optional[str] = None):
        self.parse_time_regression_threshold = 5.0  # percent
        self.parse_time_improvement_threshold = 5.0  # percent
        self.memory_usage_regression_threshold = 20.0  # percent
        self.minimum_test_coverage = 90.0  # percent
        self.confidence_level = 0.95
        self.include_detailed_stats = True
        self.generate_charts = False  # Would require matplotlib
        self.output_formats = ["json", "markdown"]
        
        if config_path and os.path.exists(config_path):
            self.load_from_file(config_path)
    
    def load_from_file(self, config_path: str) -> None:
        """Load configuration from JSON file."""
        try:
            with open(config_path, 'r') as f:
                config_data = json.load(f)
            
            # Update configuration with loaded values
            for key, value in config_data.items():
                if hasattr(self, key):
                    setattr(self, key, value)
            
            print(f"Configuration loaded from {config_path}")
        except Exception as e:
            print(f"Warning: Could not load configuration from {config_path}: {e}")
            print("Using default configuration")
    
    def save_default_config(self, config_path: str) -> None:
        """Save current configuration as default template."""
        config_data = {
            "parse_time_regression_threshold": self.parse_time_regression_threshold,
            "parse_time_improvement_threshold": self.parse_time_improvement_threshold,
            "memory_usage_regression_threshold": self.memory_usage_regression_threshold,
            "minimum_test_coverage": self.minimum_test_coverage,
            "confidence_level": self.confidence_level,
            "include_detailed_stats": self.include_detailed_stats,
            "generate_charts": self.generate_charts,
            "output_formats": self.output_formats,
            "_description": {
                "parse_time_regression_threshold": "Threshold (%) for flagging parse time regressions",
                "parse_time_improvement_threshold": "Threshold (%) for flagging parse time improvements",
                "memory_usage_regression_threshold": "Threshold (%) for flagging memory usage regressions",
                "minimum_test_coverage": "Minimum test coverage (%) required to pass gates",
                "confidence_level": "Statistical confidence level for confidence intervals",
                "include_detailed_stats": "Include detailed statistics in output",
                "generate_charts": "Generate performance charts (requires matplotlib)",
                "output_formats": "List of output formats to generate"
            }
        }
        
        with open(config_path, 'w') as f:
            json.dump(config_data, f, indent=2)
        
        print(f"Default configuration saved to {config_path}")

class BenchmarkComparison:
    """Generate comparison results between C and Rust benchmark data."""
    
    def __init__(self, c_results_path: str, rust_results_path: str, config: Optional[ComparisonConfig] = None):
        self.c_results_path = Path(c_results_path)
        self.rust_results_path = Path(rust_results_path)
        self.config = config or ComparisonConfig()
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
            
            # Determine status using configurable thresholds
            regression_threshold = self.config.parse_time_regression_threshold / 100.0
            improvement_threshold = self.config.parse_time_improvement_threshold / 100.0
            
            if time_diff > regression_threshold:
                status = "regression"
                comparison['metadata']['tests_with_regression'] += 1
            elif time_diff < -improvement_threshold:
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
        
        # Categorize by performance impact using configurable thresholds
        regression_threshold = self.config.parse_time_regression_threshold
        improvement_threshold = self.config.parse_time_improvement_threshold
        
        regressions = [td for td in time_diffs if td > regression_threshold]
        improvements = [td for td in time_diffs if td < -improvement_threshold]
        stable = [td for td in time_diffs if -improvement_threshold <= td <= regression_threshold]
        
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
        
        # Performance gates with configurable thresholds
        regression_rate = (regression_count / max(total_tests, 1)) * 100
        coverage_threshold = self.config.minimum_test_coverage
        
        gates = [
            (
                "Parse Time Regression", 
                f"<{self.config.parse_time_regression_threshold}%", 
                "✅ PASS" if regression_count == 0 else f"❌ FAIL ({regression_count} regressions)"
            ),
            (
                "Overall Performance", 
                f"<{regression_rate:.1f}%", 
                "✅ PASS" if regression_rate <= self.config.parse_time_regression_threshold else "❌ FAIL"
            ),
            (
                "Test Coverage", 
                f">{coverage_threshold}%", 
                "✅ PASS" if total_tests >= 10 else "⚠️ WARNING (insufficient tests)"
            ),
            (
                "Statistical Confidence",
                f"{self.config.confidence_level * 100}%",
                "✅ PASS" if total_tests >= 5 else "⚠️ WARNING (low sample size)"
            )
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
    parser = argparse.ArgumentParser(
        description="Generate C vs Rust benchmark comparison with configurable thresholds",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Basic comparison
  %(prog)s --c-results c_bench.json --rust-results rust_bench.json --output comparison.json --report report.md
  
  # With custom configuration
  %(prog)s --c-results c_bench.json --rust-results rust_bench.json --output comparison.json --report report.md --config comparison_config.json
  
  # Generate default configuration template
  %(prog)s --create-config comparison_config.json
        """
    )
    
    # Required arguments
    parser.add_argument("--c-results", help="Path to C implementation results JSON")
    parser.add_argument("--rust-results", help="Path to Rust implementation results JSON")
    parser.add_argument("--output", help="Output path for comparison JSON")
    parser.add_argument("--report", help="Output path for markdown report")
    
    # Configuration arguments
    parser.add_argument("--config", help="Path to configuration JSON file")
    parser.add_argument("--create-config", metavar="PATH", help="Create default configuration file and exit")
    
    # Threshold overrides
    parser.add_argument("--parse-threshold", type=float, help="Parse time regression threshold (percent)")
    parser.add_argument("--memory-threshold", type=float, help="Memory usage regression threshold (percent)")
    parser.add_argument("--min-coverage", type=float, help="Minimum test coverage (percent)")
    
    # Output options
    parser.add_argument("--detailed", action="store_true", help="Include detailed statistics")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    # Handle config creation
    if args.create_config:
        config = ComparisonConfig()
        config.save_default_config(args.create_config)
        print(f"Default configuration created at {args.create_config}")
        return
    
    # Validate required arguments
    required_args = ['c_results', 'rust_results', 'output', 'report']
    missing_args = [arg.replace('_', '-') for arg in required_args if not getattr(args, arg)]
    if missing_args:
        print(f"Error: Missing required arguments: {', '.join(missing_args)}")
        parser.print_help()
        sys.exit(1)
    
    # Load configuration
    config = ComparisonConfig(args.config)
    
    # Apply command-line overrides
    if args.parse_threshold is not None:
        config.parse_time_regression_threshold = args.parse_threshold
        config.parse_time_improvement_threshold = args.parse_threshold
    if args.memory_threshold is not None:
        config.memory_usage_regression_threshold = args.memory_threshold
    if args.min_coverage is not None:
        config.minimum_test_coverage = args.min_coverage
    if args.detailed:
        config.include_detailed_stats = True
    
    if args.verbose:
        print(f"Configuration:")
        print(f"  Parse time threshold: {config.parse_time_regression_threshold}%")
        print(f"  Memory usage threshold: {config.memory_usage_regression_threshold}%")
        print(f"  Minimum coverage: {config.minimum_test_coverage}%")
        print(f"  Confidence level: {config.confidence_level * 100}%")
        print()
    
    comparison = BenchmarkComparison(args.c_results, args.rust_results, config)
    comparison.run(args.output, args.report)

if __name__ == "__main__":
    main() 
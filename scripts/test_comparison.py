#!/usr/bin/env python3
"""
Unit tests for the benchmark comparison script.

Run with: python3 -m pytest test_comparison.py -v
"""

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# Import modules under test
from generate_comparison import BenchmarkComparison, ComparisonConfig


class TestComparisonConfig(unittest.TestCase):
    """Test the ComparisonConfig class."""
    
    def test_default_config(self):
        """Test default configuration values."""
        config = ComparisonConfig()
        
        self.assertEqual(config.parse_time_regression_threshold, 5.0)
        self.assertEqual(config.parse_time_improvement_threshold, 5.0)
        self.assertEqual(config.memory_usage_regression_threshold, 20.0)
        self.assertEqual(config.minimum_test_coverage, 90.0)
        self.assertEqual(config.confidence_level, 0.95)
        self.assertTrue(config.include_detailed_stats)
        self.assertFalse(config.generate_charts)
        self.assertEqual(config.output_formats, ["json", "markdown"])
    
    def test_load_from_file(self):
        """Test loading configuration from JSON file."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            test_config = {
                "parse_time_regression_threshold": 3.0,
                "memory_usage_regression_threshold": 15.0,
                "minimum_test_coverage": 95.0
            }
            json.dump(test_config, f)
            config_path = f.name
        
        try:
            config = ComparisonConfig(config_path)
            
            self.assertEqual(config.parse_time_regression_threshold, 3.0)
            self.assertEqual(config.memory_usage_regression_threshold, 15.0)
            self.assertEqual(config.minimum_test_coverage, 95.0)
            # Other values should remain default
            self.assertEqual(config.confidence_level, 0.95)
        finally:
            os.unlink(config_path)
    
    def test_save_default_config(self):
        """Test saving default configuration to file."""
        config = ComparisonConfig()
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            config_path = f.name
        
        try:
            config.save_default_config(config_path)
            
            # Verify file was created and contains expected data
            self.assertTrue(os.path.exists(config_path))
            
            with open(config_path, 'r') as f:
                saved_config = json.load(f)
            
            self.assertEqual(saved_config["parse_time_regression_threshold"], 5.0)
            self.assertEqual(saved_config["memory_usage_regression_threshold"], 20.0)
            self.assertTrue("_description" in saved_config)
        finally:
            if os.path.exists(config_path):
                os.unlink(config_path)


class TestBenchmarkComparison(unittest.TestCase):
    """Test the BenchmarkComparison class."""
    
    def setUp(self):
        """Set up test data."""
        self.c_data = {
            "tests": {
                "simple_test": {
                    "mean_duration_ns": 1000000,  # 1ms
                    "std_dev_ns": 100000,
                    "iterations": 100
                },
                "complex_test": {
                    "mean_duration_ns": 5000000,  # 5ms
                    "std_dev_ns": 500000,
                    "iterations": 100
                }
            }
        }
        
        self.rust_data = {
            "tests": {
                "simple_test": {
                    "mean_duration_ns": 800000,   # 0.8ms (20% faster)
                    "std_dev_ns": 80000,
                    "iterations": 100
                },
                "complex_test": {
                    "mean_duration_ns": 6000000,  # 6ms (20% slower)
                    "std_dev_ns": 600000,
                    "iterations": 100
                }
            }
        }
        
        # Create temporary files
        self.c_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
        json.dump(self.c_data, self.c_file)
        self.c_file.close()
        
        self.rust_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
        json.dump(self.rust_data, self.rust_file)
        self.rust_file.close()
        
        # Create comparison instance
        self.comparison = BenchmarkComparison(
            self.c_file.name, 
            self.rust_file.name
        )
    
    def tearDown(self):
        """Clean up temporary files."""
        os.unlink(self.c_file.name)
        os.unlink(self.rust_file.name)
    
    def test_load_data(self):
        """Test loading benchmark data from files."""
        self.comparison.load_data()
        
        self.assertEqual(self.comparison.c_data, self.c_data)
        self.assertEqual(self.comparison.rust_data, self.rust_data)
    
    def test_calculate_statistics(self):
        """Test statistical calculations."""
        values = [1.0, 2.0, 3.0, 4.0, 5.0]
        stats = self.comparison.calculate_statistics(values)
        
        self.assertEqual(stats['mean'], 3.0)
        self.assertEqual(stats['median'], 3.0)
        self.assertEqual(stats['min'], 1.0)
        self.assertEqual(stats['max'], 5.0)
        self.assertEqual(stats['count'], 5)
        self.assertAlmostEqual(stats['std_dev'], 1.5811, places=3)
    
    def test_calculate_statistics_empty(self):
        """Test statistical calculations with empty data."""
        stats = self.comparison.calculate_statistics([])
        self.assertEqual(stats, {})
    
    def test_calculate_confidence_interval(self):
        """Test confidence interval calculation."""
        values = [1.0, 2.0, 3.0, 4.0, 5.0]
        ci_low, ci_high = self.comparison.calculate_confidence_interval(values, 0.95)
        
        # Check that confidence interval is reasonable
        self.assertLess(ci_low, 3.0)  # Lower than mean
        self.assertGreater(ci_high, 3.0)  # Higher than mean
        self.assertLess(ci_high - ci_low, 3.0)  # Reasonable width
    
    def test_compare_implementations(self):
        """Test implementation comparison logic."""
        self.comparison.load_data()
        comparison = self.comparison.compare_implementations()
        
        # Check metadata
        self.assertEqual(comparison['metadata']['total_tests'], 2)
        self.assertEqual(comparison['metadata']['tests_with_improvement'], 1)
        self.assertEqual(comparison['metadata']['tests_with_regression'], 1)
        
        # Check test results
        simple_test = None
        complex_test = None
        
        for test in comparison['tests']:
            if test['name'] == 'simple_test':
                simple_test = test
            elif test['name'] == 'complex_test':
                complex_test = test
        
        self.assertIsNotNone(simple_test)
        self.assertIsNotNone(complex_test)
        
        # Simple test should be an improvement (Rust faster)
        self.assertEqual(simple_test['comparison']['status'], 'improvement')
        self.assertLess(simple_test['comparison']['time_difference'], 0)
        
        # Complex test should be a regression (Rust slower)
        self.assertEqual(complex_test['comparison']['status'], 'regression')
        self.assertGreater(complex_test['comparison']['time_difference'], 0)
    
    def test_compare_with_custom_thresholds(self):
        """Test comparison with custom threshold configuration."""
        config = ComparisonConfig()
        config.parse_time_regression_threshold = 25.0  # Higher threshold
        config.parse_time_improvement_threshold = 25.0
        
        comparison_custom = BenchmarkComparison(
            self.c_file.name,
            self.rust_file.name,
            config
        )
        
        comparison_custom.load_data()
        comparison = comparison_custom.compare_implementations()
        
        # With higher thresholds, both tests should be within tolerance
        for test in comparison['tests']:
            self.assertEqual(test['comparison']['status'], 'within_tolerance')
    
    def test_generate_summary_statistics(self):
        """Test summary statistics generation."""
        self.comparison.load_data()
        comparison = self.comparison.compare_implementations()
        
        summary = comparison['summary']
        
        # Check that summary contains expected keys
        required_keys = [
            'overall_performance',
            'performance_distribution', 
            'test_coverage'
        ]
        
        for key in required_keys:
            self.assertIn(key, summary)
        
        # Check performance distribution
        perf_dist = summary['performance_distribution']
        self.assertEqual(perf_dist['regressions_count'], 1)
        self.assertEqual(perf_dist['improvements_count'], 1)
        
        # Check test coverage
        coverage = summary['test_coverage']
        self.assertEqual(coverage['total_tests'], 2)
    
    def test_generate_markdown_report(self):
        """Test markdown report generation."""
        self.comparison.load_data()
        comparison = self.comparison.compare_implementations()
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as f:
            report_path = f.name
        
        try:
            self.comparison.generate_markdown_report(comparison, report_path)
            
            # Verify report file was created
            self.assertTrue(os.path.exists(report_path))
            
            # Verify report content
            with open(report_path, 'r') as f:
                content = f.read()
            
            # Check for expected sections
            self.assertIn("# Tree-sitter Perl Benchmark Comparison Report", content)
            self.assertIn("## Executive Summary", content)
            self.assertIn("## Detailed Test Results", content)
            self.assertIn("## Performance Gates Status", content)
            
            # Check for test data
            self.assertIn("simple_test", content)
            self.assertIn("complex_test", content)
            
        finally:
            if os.path.exists(report_path):
                os.unlink(report_path)


class TestIntegration(unittest.TestCase):
    """Integration tests for the complete workflow."""
    
    def test_end_to_end_workflow(self):
        """Test the complete comparison workflow."""
        # Create realistic test data
        c_data = {
            "metadata": {
                "generated_at": "2023-01-01T00:00:00Z",
                "parser_version": "1.0.0",
                "total_tests": 3
            },
            "tests": {
                "small_file": {
                    "mean_duration_ns": 100000,  # 0.1ms
                    "std_dev_ns": 10000,
                    "iterations": 100
                },
                "medium_file": {
                    "mean_duration_ns": 1000000,  # 1ms
                    "std_dev_ns": 100000,
                    "iterations": 100
                },
                "large_file": {
                    "mean_duration_ns": 10000000,  # 10ms
                    "std_dev_ns": 1000000,
                    "iterations": 100
                }
            }
        }
        
        rust_data = {
            "metadata": {
                "generated_at": "2023-01-01T00:00:00Z",
                "parser_version": "1.0.0",
                "total_tests": 3
            },
            "tests": {
                "small_file": {
                    "mean_duration_ns": 95000,   # Slight improvement
                    "std_dev_ns": 9500,
                    "iterations": 100
                },
                "medium_file": {
                    "mean_duration_ns": 980000,  # Slight improvement
                    "std_dev_ns": 98000,
                    "iterations": 100
                },
                "large_file": {
                    "mean_duration_ns": 11000000, # Slight regression
                    "std_dev_ns": 1100000,
                    "iterations": 100
                }
            }
        }
        
        # Create temporary files
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as c_file:
            json.dump(c_data, c_file)
            c_path = c_file.name
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as rust_file:
            json.dump(rust_data, rust_file)
            rust_path = rust_file.name
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as out_file:
            output_path = out_file.name
        
        with tempfile.NamedTemporaryFile(mode='w', suffix='.md', delete=False) as report_file:
            report_path = report_file.name
        
        try:
            # Create comparison with custom config
            config = ComparisonConfig()
            comparison = BenchmarkComparison(c_path, rust_path, config)
            comparison.run(output_path, report_path)
            
            # Verify outputs exist
            self.assertTrue(os.path.exists(output_path))
            self.assertTrue(os.path.exists(report_path))
            
            # Verify output content
            with open(output_path, 'r') as f:
                output_data = json.load(f)
            
            self.assertIn('metadata', output_data)
            self.assertIn('tests', output_data)
            self.assertIn('summary', output_data)
            self.assertEqual(len(output_data['tests']), 3)
            
            # Verify report content
            with open(report_path, 'r') as f:
                report_content = f.read()
            
            self.assertIn('small_file', report_content)
            self.assertIn('medium_file', report_content)
            self.assertIn('large_file', report_content)
            
        finally:
            # Clean up
            for path in [c_path, rust_path, output_path, report_path]:
                if os.path.exists(path):
                    os.unlink(path)


if __name__ == '__main__':
    # Run tests
    unittest.main(verbosity=2)
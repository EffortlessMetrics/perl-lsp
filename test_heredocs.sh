#!/bin/bash

# Script to run each heredoc integration test individually

cd /home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs

echo "Running heredoc integration tests individually..."
echo "=============================================="

tests=(
    "test_basic_heredoc"
    "test_interpolated_heredoc"
    "test_multiple_heredocs"
    "test_indented_heredoc"
    "test_heredoc_in_expression"
    "test_heredoc_with_special_chars"
    "test_heredoc_with_empty_lines"
    "test_heredoc_terminator_in_content"
    "test_heredoc_preprocessing"
    "test_heredoc_with_slash_disambiguation"
    "test_complex_heredoc_scenario"
    "test_heredoc_error_recovery"
)

for test in "${tests[@]}"; do
    echo ""
    echo "Running: $test"
    echo "----------------------------------------"
    
    # Run test with timeout to catch stack overflows
    timeout 10s cargo test --features pure-rust heredoc_integration_tests::$test -- --exact --nocapture 2>&1
    
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo "✓ PASSED"
    elif [ $exit_code -eq 124 ]; then
        echo "✗ TIMEOUT (likely stack overflow)"
    else
        echo "✗ FAILED (exit code: $exit_code)"
    fi
done

echo ""
echo "=============================================="
echo "Test run complete"
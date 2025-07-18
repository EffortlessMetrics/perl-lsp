#!/bin/bash

cd /home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs

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

echo "Heredoc Integration Test Results"
echo "================================"

for test in "${tests[@]}"; do
    echo -n "$test: "
    
    # Run test and capture output
    output=$(timeout 5s cargo test --features pure-rust heredoc_integration_tests::$test -- --exact 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo "✓ PASSED"
    elif [ $exit_code -eq 124 ]; then
        echo "✗ STACK OVERFLOW"
    else
        # Check for specific error patterns
        if echo "$output" | grep -q "stack overflow"; then
            echo "✗ STACK OVERFLOW"
        elif echo "$output" | grep -q "panicked at"; then
            error=$(echo "$output" | grep "panicked at" | head -1 | sed 's/.*panicked at/panicked at/')
            echo "✗ FAILED - $error"
        elif echo "$output" | grep -q "FAILED"; then
            echo "✗ FAILED"
        else
            echo "✗ FAILED (exit code: $exit_code)"
        fi
    fi
done
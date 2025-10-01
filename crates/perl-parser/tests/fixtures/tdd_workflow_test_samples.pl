#!/usr/bin/perl
# TDD Workflow Test Samples for tdd_workflow.rs validation
# Tests the Red-Green-Refactor cycle functionality

package TDD::WorkflowSamples;
use strict;
use warnings;
use Test::More;

# RED phase: Test that should initially fail
sub failing_test_example {
    # This function doesn't exist yet - TDD workflow should generate it
    my $result = calculate_average([1, 2, 3, 4, 5]);
    is($result, 3, "Calculate average should return 3");
}

# GREEN phase: Minimal implementation to make test pass
sub calculate_average {
    my ($numbers) = @_;
    return unless @$numbers;

    my $sum = 0;
    $sum += $_ for @$numbers;
    return $sum / @$numbers;
}

# REFACTOR phase: Function that can be improved
sub unrefactored_string_processor {
    my ($input) = @_;

    # This implementation has code duplication and complexity
    my $result = $input;

    # Remove whitespace (duplicated logic)
    $result =~ s/^\s+//;
    $result =~ s/\s+$//;

    # Convert to lowercase
    $result = lc($result);

    # Remove special characters (duplicated pattern)
    $result =~ s/[^a-z0-9\s]//g;

    # Normalize spaces (duplicated logic)
    $result =~ s/^\s+//;
    $result =~ s/\s+$//;
    $result =~ s/\s+/ /g;

    return $result;
}

# Function with parameters for testing parameter handling in TDD workflow
sub multi_parameter_function {
    my ($name, $age, $email, $preferences) = @_;

    return {
        display_name => "$name ($age)",
        contact => $email,
        settings => $preferences,
    };
}

# Function with complex signature for testing signature parsing
sub complex_signature_function {
    my ($self, $required_param, $optional_param, %options) = @_;

    my $defaults = {
        debug => 0,
        verbose => 1,
        format => 'json',
    };

    my $config = { %$defaults, %options };

    return process_with_config($required_param, $optional_param, $config);
}

# Test case generation helpers
sub generate_test_cases {
    my ($function_name) = @_;

    return [
        {
            name => "${function_name}_basic_test",
            input => ['test_input'],
            expected => 'expected_output',
        },
        {
            name => "${function_name}_edge_case",
            input => [undef],
            expected => undef,
        },
        {
            name => "${function_name}_empty_input",
            input => [''],
            expected => '',
        },
    ];
}

# Coverage testing function
sub coverage_test_target {
    my ($condition, $data) = @_;

    if ($condition eq 'branch_a') {
        return process_branch_a($data);
    } elsif ($condition eq 'branch_b') {
        return process_branch_b($data);
    } else {
        return default_processing($data);
    }
}

sub process_branch_a {
    my ($data) = @_;
    return "Processed A: $data";
}

sub process_branch_b {
    my ($data) = @_;
    return "Processed B: $data";
}

sub default_processing {
    my ($data) = @_;
    return "Default: $data";
}

# Integration test sample
sub integration_test_function {
    my ($external_service, $api_key) = @_;

    # This would typically call external services
    # For TDD, we'd mock these dependencies
    return call_external_api($external_service, $api_key);
}

sub call_external_api {
    my ($service, $key) = @_;
    # Mock implementation for testing
    return { status => 'success', data => 'mock_data' };
}

1;

__END__

=head1 NAME

TDD::WorkflowSamples - Test samples for TDD workflow validation

=head1 DESCRIPTION

This module contains various Perl functions designed to test different
aspects of the TDD workflow implementation:

- Red phase: Tests that should initially fail
- Green phase: Minimal implementations
- Refactor phase: Code that needs improvement
- Parameter handling testing
- Coverage analysis targets

=head1 FUNCTIONS

=head2 calculate_average(\@numbers)

Calculates the arithmetic mean of an array of numbers.

=head2 multi_parameter_function($name, $age, $email, $preferences)

Function with multiple parameters for testing parameter parsing.

=head2 coverage_test_target($condition, $data)

Function with multiple branches for testing code coverage analysis.

=cut
#!/usr/bin/perl
# Test fixtures for Issue #146 - Architectural Integrity Repair
# These Perl code samples test TDD workflow and refactoring functionality

package TestModule::Issue146;
use strict;
use warnings;
use Test::More;

# Test subroutine for TDD workflow testing
sub calculate_fibonacci {
    my ($n) = @_;
    return 0 if $n <= 0;
    return 1 if $n == 1;
    return calculate_fibonacci($n - 1) + calculate_fibonacci($n - 2);
}

# Complex subroutine that needs refactoring (for refactoring.rs testing)
sub complex_data_processor {
    my ($data, $options, $flags, $config, $metadata) = @_;

    # This subroutine has too many parameters (refactoring opportunity)
    my $result = {};

    if ($options->{process_all}) {
        for my $item (@$data) {
            if ($flags->{validate}) {
                next unless validate_item($item, $config);
            }

            my $processed = process_single_item($item, $options, $metadata);
            push @{$result->{processed}}, $processed;
        }
    }

    return $result;
}

# Subroutine with duplicate code (refactoring opportunity)
sub process_user_data {
    my ($user_data) = @_;

    # Validation logic (duplicated below)
    unless ($user_data->{name}) {
        warn "Missing user name";
        return;
    }
    unless ($user_data->{email}) {
        warn "Missing user email";
        return;
    }

    # Process user
    return format_user_data($user_data);
}

# Another subroutine with duplicate validation (refactoring opportunity)
sub process_admin_data {
    my ($admin_data) = @_;

    # Same validation logic as above (should be extracted)
    unless ($admin_data->{name}) {
        warn "Missing admin name";
        return;
    }
    unless ($admin_data->{email}) {
        warn "Missing admin email";
        return;
    }

    # Process admin
    return format_admin_data($admin_data);
}

# Simple subroutine for basic TDD testing
sub add_numbers {
    my ($a, $b) = @_;
    return $a + $b;
}

# Subroutine that needs performance optimization
sub inefficient_search {
    my ($array, $target) = @_;

    # Inefficient linear search (could be optimized)
    for my $i (0..$#$array) {
        for my $j (0..$#$array) {
            if ($array->[$i] eq $target && $i == $j) {
                return $i;
            }
        }
    }
    return -1;
}

# Package-scoped variables for testing workspace indexing
our $GLOBAL_CONFIG = {
    debug_mode => 1,
    log_level => 'INFO',
};

my $private_cache = {};

# Test package inheritance for cross-file navigation testing
package TestModule::Issue146::Child;
use parent 'TestModule::Issue146';

sub child_specific_method {
    my ($self, $param) = @_;
    return $self->add_numbers($param, 10);
}

# Test nested package for complex navigation scenarios
package TestModule::Issue146::Nested::Deep;

sub deeply_nested_function {
    my ($data) = @_;
    return TestModule::Issue146::calculate_fibonacci($data);
}

1;

__END__

=head1 NAME

TestModule::Issue146 - Test fixtures for architectural integrity repair

=head1 DESCRIPTION

This module provides test fixtures for validating the restoration of
tdd_workflow.rs and refactoring.rs modules in the Perl LSP parser.

=head1 FUNCTIONS

=head2 calculate_fibonacci($n)

Calculates the nth Fibonacci number. Used for TDD workflow testing.

=head2 complex_data_processor($data, $options, $flags, $config, $metadata)

Complex function with too many parameters, used for refactoring testing.

=head2 add_numbers($a, $b)

Simple addition function for basic TDD testing.

=head1 AUTHOR

Perl LSP Test Suite

=cut
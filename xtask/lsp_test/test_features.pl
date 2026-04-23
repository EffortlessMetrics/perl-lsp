#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Function for call hierarchy testing
sub process_data {
    my ($data) = @_;
    validate_input($data);
    transform_data($data);
    return calculate_result($data);
}

sub validate_input {
    my ($input) = @_;
    die "Invalid input" unless defined $input;
}

sub transform_data {
    my ($data) = @_;
    $data->{transformed} = 1;
}

sub calculate_result {
    my ($data) = @_;
    return $data->{value} * 2;
}

# Test functions
sub test_basic_math {
    is(2 + 2, 4, "Basic addition works");
}

sub test_string_operations {
    my $str = "Hello";
    is(length($str), 5, "String length is correct");
}

# Call the functions to demonstrate call hierarchy
my $result = process_data({ value => 10 });

# Run tests
test_basic_math();
test_string_operations();

done_testing();

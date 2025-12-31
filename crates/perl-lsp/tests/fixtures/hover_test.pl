#!/usr/bin/perl
use strict;
use warnings;

# Test file for hover functionality
# Contains various Perl constructs for hover testing

package HoverTest;

# Custom subroutine for testing hover at line 10, character 15
sub calculate_sum {
    my ($a, $b) = @_;
    return $a + $b;
}

# Using built-in function 'join' at line 16, character 20
my $joined = join(", ", ("a", "b", "c"));

# Using custom function
my $result = calculate_sum(10, 20);

# More subroutines for folding tests
sub process_data {
    my ($data) = @_;
    print "Processing: $data\n";
}

sub format_output {
    my ($format, @args) = @_;
    return sprintf($format, @args);
}

1;

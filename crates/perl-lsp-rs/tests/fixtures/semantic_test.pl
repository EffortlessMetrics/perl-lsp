#!/usr/bin/perl
use strict;
use warnings;

# Test file for semantic tokens functionality
# Contains various Perl constructs for semantic highlighting

package SemanticTest;

# Variables with different sigils
my $scalar = "hello";
my @array = (1, 2, 3);
my %hash = (key => "value");

# Subroutine with parameters
sub process_items {
    my ($input, @rest) = @_;
    foreach my $item (@rest) {
        print "$item\n";
    }
    return length($input);
}

# Control structures for semantic highlighting
if ($scalar eq "hello") {
    my $local = "world";
    print join(" ", $scalar, $local), "\n";
}

# Method call syntax
my $result = SemanticTest->process_items("test", 1, 2, 3);

# Regular expressions
if ($scalar =~ /^hel/) {
    print "Matched!\n";
}

1;

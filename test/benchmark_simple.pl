#!/usr/bin/perl

# Simple Perl file for benchmarking
use strict;
use warnings;

my $name = "World";
print "Hello, $name!\n";

# Basic arithmetic
my $result = 2 + 2;
print "2 + 2 = $result\n";

# Array operations
my @numbers = (1, 2, 3, 4, 5);
foreach my $num (@numbers) {
    print "Number: $num\n";
}

# Hash operations
my %config = (
    'host' => 'localhost',
    'port' => 8080,
    'debug' => 1
);

# Subroutine definition
sub greet {
    my ($person) = @_;
    return "Hello, $person!";
}

# Function call
my $greeting = greet("Alice");
print "$greeting\n";

# Conditional statements
if ($result == 4) {
    print "Math is working!\n";
} else {
    print "Something is wrong!\n";
}

# Loop with condition
for (my $i = 0; $i < 3; $i++) {
    print "Iteration $i\n";
}

# Regular expressions
my $text = "Hello World";
if ($text =~ /World/) {
    print "Found 'World' in text\n";
}

# File operations
open(my $fh, '<', $0) or die "Cannot open file: $!";
my $line_count = 0;
while (<$fh>) {
    $line_count++;
}
close($fh);
print "This file has $line_count lines\n";

# Package declaration
package MyModule;

sub new {
    my $class = shift;
    my $self = {};
    bless $self, $class;
    return $self;
}

1; # Return true 
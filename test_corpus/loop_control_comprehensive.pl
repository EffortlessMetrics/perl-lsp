#!/usr/bin/env perl
# Test: Comprehensive Loop Control Statements
# Impact: Ensures parser handles all loop control constructs (next, last, redo, continue)
# NodeKinds: LoopControl (next, last, redo, continue)

use strict;
use warnings;

# Basic next in foreach
foreach my $i (1..10) {
    next if $i % 2 == 0;  # Skip even numbers
    print "$i ";
}

# Basic last in while
my $count = 0;
while ($count < 100) {
    last if $count == 5;  # Exit loop at 5
    $count++;
}

# Basic redo in for
for (my $i = 0; $i < 3; $i++) {
    print "Iteration $i\n";
    redo if $i == 0;  # Redo first iteration
}

# Continue block with foreach
foreach my $item (1..5) {
    print "Processing $item\n";
} continue {
    print "Continue block for $item\n";
}

# Continue block with while
my $j = 0;
while ($j < 3) {
    print "While iteration $j\n";
    $j++;
} continue {
    print "Continue after while\n";
}

# Continue block with until
my $k = 0;
until ($k >= 3) {
    print "Until iteration $k\n";
    $k++;
} continue {
    print "Continue after until\n";
}

# Labeled loops with controls
OUTER: for my $x (1..3) {
    INNER: for my $y (1..3) {
        next OUTER if $x == $y;  # Skip to outer loop
        last INNER if $y == 2;    # Exit inner loop
        redo INNER if $x == 1 && $y == 1;  # Redo inner loop
        print "$x,$y ";
    }
}

# Loop control in nested contexts
foreach my $outer (1..3) {
    foreach my $inner (1..3) {
        if ($inner == 2) {
            next;  # Skip inner iteration
        }
        if ($outer == 2 && $inner == 3) {
            last;  # Exit outer loop
        }
        print "$outer:$inner ";
    }
}

# Continue with complex logic
my @results;
for my $value (1..5) {
    push @results, $value * 2;
} continue {
    # This runs after each iteration, including the last one
    print "Processed value $value\n";
}

# Loop control with conditional expressions
foreach my $num (1..10) {
    $num % 2 == 0 ? next : print "$num ";  # Next on even, print on odd
}

# Loop control in do-while (emulated with while + last)
my $do_while_var = 0;
{
    do {
        print "Do-while iteration $do_while_var\n";
        $do_while_var++;
        last if $do_while_var > 3;
    } while (0);
}

# Complex continue with multiple statements
my $total = 0;
for my $i (1..5) {
    $total += $i;
    print "Added $i\n";
} continue {
    print "Running continue block\n";
    print "Current total: $total\n";
}

print "\nAll loop control tests completed\n";
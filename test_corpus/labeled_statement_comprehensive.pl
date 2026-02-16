#!/usr/bin/env perl
# Test: LabeledStatement NodeKind
# Impact: Ensures parser handles labeled loops and blocks
# NodeKinds: LabeledStatement
# 
# This file tests the parser's ability to handle:
# 1. Labeled while loops
# 2. Labeled for loops
# 3. Labeled foreach loops
# 4. Labeled blocks
# 5. Nested labeled statements
# 6. Labeled statements with next/last/redo
# 7. Complex labeled statement scenarios

use strict;
use warnings;

# Basic labeled while loop
OUTER_WHILE: while (1) {
    print "In outer while loop\n";
    last OUTER_WHILE;  # Exit the labeled loop
}

# Labeled for loop (C-style)
FOR_LOOP: for (my $i = 0; $i < 5; $i++) {
    print "For loop iteration: $i\n";
    next FOR_LOOP if $i == 2;  # Skip iteration 3
    print "  Processed: $i\n";
}

# Labeled foreach loop
FOREACH_LOOP: foreach my $item (1..5) {
    print "Foreach item: $item\n";
    last FOREACH_LOOP if $item == 3;  # Exit after item 3
}

# Labeled block
LABELED_BLOCK: {
    print "In labeled block\n";
    print "Block continues\n";
    # No explicit exit needed for blocks
}

# Nested labeled loops
OUTER_NESTED: for my $i (1..3) {
    print "Outer loop: $i\n";
    
    INNER_NESTED: for my $j (1..3) {
        print "  Inner loop: $j\n";
        
        if ($i == 2 && $j == 2) {
            last OUTER_NESTED;  # Exit both loops
        }
    }
    
    print "After inner loop (i=$i)\n";
}

# Labeled statement with conditional next
CONDITIONAL_NEXT: for my $num (1..10) {
    next CONDITIONAL_NEXT if $num % 2 == 0;  # Skip even numbers
    print "Odd number: $num\n";
}

# Labeled statement with conditional last
CONDITIONAL_LAST: for my $num (1..10) {
    print "Number: $num\n";
    last CONDITIONAL_LAST if $num > 5;  # Exit after 5
}

# Labeled statement with redo
REDO_LOOP: for my $count (1..5) {
    print "Count: $count\n";
    
    if ($count == 2) {
        $count--;  # Decrement to repeat this iteration
        redo REDO_LOOP;
    }
}

# Complex labeled statement scenarios

# Scenario 1: Labeled statement with error handling
ERROR_HANDLING: while (1) {
    eval {
        # Simulate operation that might fail
        die "Simulated error" if rand() < 0.3;
        print "Operation succeeded\n";
        1;
    } or do {
        warn "Error occurred: $@";
        last ERROR_HANDLING;  # Exit on error
    };
    
    last ERROR_HANDLING;  # Exit after one iteration
}

# Scenario 2: Labeled statement with file processing
FILE_PROCESSING: while (my $filename = shift @ARGV) {
    open my $fh, '<', $filename or do {
        warn "Cannot open $filename: $!";
        next FILE_PROCESSING;  # Skip to next file
    };
    
    while (my $line = <$fh>) {
        chomp $line;
        next FILE_PROCESSING if $line =~ /^\s*#/;  # Skip comments
        last FILE_PROCESSING if $line =~ /^__END__/;  # Stop at end marker
        print "Processing: $line\n";
    }
    
    close $fh;
}

# Scenario 3: Labeled statement with data validation
VALIDATION_LOOP: foreach my $data (@ARGV) {
    unless (defined $data && length $data) {
        warn "Invalid data: undefined or empty";
        next VALIDATION_LOOP;
    }
    
    unless ($data =~ /^\d+$/) {
        warn "Invalid data: not numeric ($data)";
        next VALIDATION_LOOP;
    }
    
    print "Valid data: $data\n";
}

# Scenario 4: Labeled statement with complex control flow
COMPLEX_FLOW: for my $outer (1..3) {
    print "Outer: $outer\n";
    
    INNER_COMPLEX: for my $inner (1..3) {
        print "  Inner: $inner\n";
        
        MIDDLE_COMPLEX: {
            if ($outer == 2 && $inner == 2) {
                print "    Taking middle exit\n";
                last MIDDLE_COMPLEX;
            }
            
            if ($outer == 3) {
                print "    Taking inner exit\n";
                last INNER_COMPLEX;
            }
            
            print "    Normal processing\n";
        }
        
        print "  After middle block (outer=$outer, inner=$inner)\n";
    }
    
    print "After inner loop (outer=$outer)\n";
}

# Scenario 5: Labeled statement with recursion simulation
SIMULATED_RECURSION: {
    my $depth = 0;
    my $max_depth = 3;
    
    RECURSIVE_BLOCK: {
        print "Depth: $depth\n";
        
        if ($depth >= $max_depth) {
            print "Maximum depth reached\n";
            last RECURSIVE_BLOCK;
        }
        
        $depth++;
        redo RECURSIVE_BLOCK;  # Simulate recursive call
    }
}

# Labeled statements with different loop types

# Type 1: Labeled do-while simulation
DO_WHILE_SIM: {
    my $count = 0;
    
    do {
        print "Do-while iteration: $count\n";
        $count++;
        
        if ($count > 3) {
            last DO_WHILE_SIM;
        }
    } while ($count < 10);
}

# Type 2: Labeled until simulation
UNTIL_SIM: for (;;) {
    my $condition = int(rand(10));
    print "Until simulation: $condition\n";
    
    last UNTIL_SIM if $condition > 7;
}

# Type 3: Labeled grep/map simulation
GREP_MAP_SIM: foreach my $item (1..10) {
    my $result = $item * 2;
    
    next GREP_MAP_SIM if $result % 3 == 0;  # Skip multiples of 3
    print "Grep/map result: $result\n";
    
    last GREP_MAP_SIM if $result > 15;  # Exit when result is large
}

# Labeled statements with special variables

# Special 1: Labeled loop with $.
LINE_NUMBER: while (<DATA>) {
    print "Line $.: $_";  # $. is current line number
    last LINE_NUMBER if $. > 5;  # Process only first 5 lines
}

# Special 2: Labeled loop with $1, $2, etc.
REGEX_LOOP: foreach my $text ('abc123', 'def456', 'ghi789') {
    if ($text =~ /(\w+)(\d+)/) {
        print "Matched: word=$1, digits=$2\n";
        last REGEX_LOOP if $2 > 500;  # Exit when digits are large
    }
}

# Special 3: Labeled loop with @ARGV
ARGV_LOOP: while (my $arg = shift @ARGV) {
    print "Processing argument: $arg\n";
    next ARGV_LOOP if $arg =~ /^-/;  # Skip options
    last ARGV_LOOP if $arg eq 'stop';  # Stop on 'stop'
}

# Labeled statements with subroutines

# Subroutine 1: Labeled statement in subroutine
sub labeled_subroutine {
    my ($max) = @_;
    
    SUB_LABEL: for my $i (1..$max) {
        print "Sub routine iteration: $i\n";
        next SUB_LABEL if $i % 2 == 0;  # Skip even numbers
        return $i if $i > 5;  # Return early
    }
    
    return "completed";
}

# Subroutine 2: Subroutine with labeled error handling
sub safe_operation {
    my ($operation) = @_;
    
    OPERATION_LABEL: {
        eval {
            # Simulate operation
            die "Operation failed" if $operation eq 'fail';
            print "Operation succeeded: $operation\n";
            1;
        } or do {
            warn "Error in operation: $@";
            last OPERATION_LABEL;  # Exit labeled block
        };
        
        print "Operation cleanup\n";
    }
    
    return $operation ne 'fail';
}

# Labeled statements with object-oriented patterns

# Object pattern 1: Labeled statement for method chaining
METHOD_CHAIN: foreach my $method (qw(connect authenticate query disconnect)) {
    print "Calling method: $method\n";
    
    if ($method eq 'authenticate' && rand() < 0.3) {
        print "Authentication failed\n";
        last METHOD_CHAIN;  # Stop chain on auth failure
    }
    
    if ($method eq 'disconnect') {
        print "Method chain completed\n";
    }
}

# Object pattern 2: Labeled statement for state machine
STATE_MACHINE: {
    my $state = 'initial';
    
    STATE_LOOP: while ($state ne 'final') {
        print "Current state: $state\n";
        
        if ($state eq 'initial') {
            $state = 'processing';
        } elsif ($state eq 'processing') {
            $state = rand() < 0.5 ? 'success' : 'error';
        } elsif ($state eq 'success') {
            $state = 'final';
        } elsif ($state eq 'error') {
            print "Error state reached\n";
            last STATE_LOOP;
        }
    }
    
    print "State machine finished\n";
}

# Edge cases and error handling

# Edge case 1: Labeled statement with empty block
EMPTY_LABEL: {
    # Empty labeled block
    print "Empty label executed\n";
}

# Edge case 2: Labeled statement with immediate exit
IMMEDIATE_EXIT: {
    last IMMEDIATE_EXIT;  # Exit immediately
    print "This should not print\n";
}

# Edge case 3: Labeled statement with multiple exit points
MULTIPLE_EXIT: for my $i (1..10) {
    print "Multiple exit test: $i\n";
    
    next MULTIPLE_EXIT if $i < 3;  # Skip first 2
    last MULTIPLE_EXIT if $i > 5;  # Exit after 5
    redo MULTIPLE_EXIT if $i == 4;  # Repeat iteration 4
    
    print "Normal processing: $i\n";
}

# Performance considerations

# Performance 1: Labeled statement with early exit
PERFORMANCE_TEST: for my $i (1..1000000) {
    last PERFORMANCE_TEST if $i > 100;  # Early exit
    # Process first 100 items only
}

# Performance 2: Labeled statement with loop optimization
OPTIMIZED_LOOP: foreach my $item (@large_array) {
    next OPTIMIZED_LOOP unless defined $item;  # Skip undef items
    next OPTIMIZED_LOOP unless length $item;  # Skip empty strings
    
    # Process valid items only
    process_valid_item($item);
}

# Cross-file interaction simulation
# This demonstrates how labeled statements might interact with other files

# Cross-file 1: Labeled statement with module use
MODULE_USE: {
    # use Some::Module;  # Would use external module
    # my $result = Some::Module::function();
    # print "Module result: $result\n";
    print "Cross-file module use simulation\n";
}

# Cross-file 2: Labeled statement with require
REQUIRE_LABEL: {
    # require 'external.pl';  # Would require external file
    # external_function();  # Would call function from external file
    print "Cross-file require simulation\n";
}

__DATA__
Line 1 from DATA
Line 2 from DATA
Line 3 from DATA
Line 4 from DATA
Line 5 from DATA
Line 6 from DATA
Line 7 from DATA
Line 8 from DATA
Line 9 from DATA
Line 10 from DATA

print "LabeledStatement tests completed successfully\n";
#!/usr/bin/perl
# Test fixture for diagnostics

# Missing 'use strict' - should suggest adding it
my $global_var = "test";

# Undefined variable (would be caught with 'use strict')
print $undefined_var;

# Unused variable warning
my $unused = 42;

sub test_func {
    # Variable shadowing
    my $global_var = "local";
    
    # Typo in variable name
    my $important_data = "data";
    print $imporant_data;  # Typo: should be $important_data
    
    return $global_var;
}

# Missing semicolon
my $x = 5
my $y = 10;

# Syntax error - unclosed string
my $str = "unclosed string
my $next = "line";

1;
#!/usr/bin/env perl
use strict;
use warnings;

# Test 1: Undeclared variable
print "Test 1: Undeclared variable\n";
{
    use strict;
    my $declared = 10;
    print $undeclared;  # This should be an error
}

# Test 2: Multiple scope levels
print "\nTest 2: Multiple scope levels\n";
{
    my $outer = 1;
    {
        my $middle = 2;
        {
            my $inner = 3;
            print $outer, $middle, $inner;
        }
        print $middle;  # $inner not accessible here
    }
}

# Test 3: Package variables
print "\nTest 3: Package variables\n";
{
    package MyPackage;
    our $package_var = 10;
    my $lexical_var = 20;
    
    sub get_package { return $package_var; }
    sub get_lexical { return $lexical_var; }
}
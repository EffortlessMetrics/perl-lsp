#!/usr/bin/env perl

# Test return or die with if modifier
sub test {
    my $x = shift;
    my $error = shift;
    
    return $x or die if $error;
    
    return 42;
}

print test(5, 0), "\n";
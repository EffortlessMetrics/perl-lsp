#!/usr/bin/env perl

# Test return or die without modifier
sub test {
    my $x = shift;
    
    return $x or die;
    
    return 42;
}

print test(5), "\n";
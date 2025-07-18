#!/usr/bin/perl

sub test_function {
    my $x = 1;
    
=pod
This is documentation
=cut
    
    my $y = 2;
    return $x + $y;
}

my $result = test_function();
print "Result: $result\n";
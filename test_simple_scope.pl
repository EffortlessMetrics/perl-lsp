#!/usr/bin/env perl
my $outer = 1;
{
    my $middle = 2;
    {
        my $inner = 3;
        print $outer, $middle, $inner;
    }
    print $middle;
}
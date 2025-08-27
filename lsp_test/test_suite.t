#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Test basic functionality
ok(1, "True is true");
is(2 + 2, 4, "Math works");

# Test string operations
my $str = "test";
like($str, qr/test/, "Regex matching works");

# Test array operations
my @arr = (1, 2, 3);
is(scalar(@arr), 3, "Array has correct size");

done_testing();

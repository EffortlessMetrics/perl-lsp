package Fuzz::Basic;
use strict;
use warnings;
my $value = 42;
sub add { my ($left, $right) = @_; return $left + $right + $value; }
add(1, 2);

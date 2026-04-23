package My::Module;

use strict;
use warnings;

my $scalar_var = 42;
my @array_var = (1, 2, 3);
my %hash_var = (key => 'value');

sub my_function {
    my ($x, $y) = @_;
    return $x + $y;
}

sub another_method {
    return "hello";
}

1;

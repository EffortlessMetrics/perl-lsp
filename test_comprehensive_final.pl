#!/usr/bin/perl
use strict;
use warnings;

# Variables and operators
my $x = 5 + 3 * 2;
my $str = "Hello" . " World";
my @range = (1..10);

# Control flow
if ($x > 10) {
    print "Big\n";
} elsif ($x > 5) {
    print "Medium\n";
} else {
    print "Small\n";
}

# Loops
for (my $i = 0; $i < 5; $i++) {
    print "$i ";
}

foreach my $num (@range) {
    print "Number: $num\n";
}

# Subroutines
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

# Anonymous sub
my $multiply = sub {
    return $_[0] * $_[1];
};

# Array/hash access
my @array = (10, 20, 30);
my %hash = (key => 'value');
my $elem = $array[1];
my $val = $hash{key};

# Method calls
my $obj = Package->new();
$obj->method();

# Complex expressions
my $result = $multiply->(4, 5) + add(1, 2);

1;
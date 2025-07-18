#!/usr/bin/perl
use strict;
use warnings;

=pod
This is a comprehensive test of the Pure Rust Perl parser.
It demonstrates all the features that are currently working.
=cut

# Variables and basic operations
my $x = 42;
my $y = 3.14;
my $str = "Hello" . " World";
my @array = (1, 2, 3, 4, 5);
my %hash = (
    name => 'John',
    age => 30,
    city => 'New York'
);

# Operators and expressions
my $sum = $x + $y;
my $product = $x * $y;
my $power = 2 ** 10;
my $concat = "foo" . "bar";
my @range = (1..10);

# Control flow
if ($x > 0) {
    print "x is positive\n";
} elsif ($x < 0) {
    print "x is negative\n";
} else {
    print "x is zero\n";
}

unless ($y == 0) {
    my $div = $x / $y;
}

# Loops
for (my $i = 0; $i < 5; $i++) {
    print "i = $i\n";
}

foreach my $item (@array) {
    print "Item: $item\n";
}

while ($x > 0) {
    $x--;
    last if $x == 20;
}

# Subroutines
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

sub multiply($$$) {
    my ($x, $y, $z) = @_;
    return $x * $y * $z;
}

# Anonymous subroutines
my $square = sub {
    my $n = shift;
    return $n * $n;
};

my $cube = sub ($x) { return $x ** 3; };

# Array and hash access
my $first = $array[0];
my $last = $array[-1];
my $name = $hash{name};
my $key = 'age';
my $age = $hash{$key};

# Complex access chains
my $data = {
    users => [
        { name => 'Alice', scores => [90, 85, 92] },
        { name => 'Bob', scores => [78, 82, 88] }
    ]
};
my $alice_score = $data->{users}[0]{scores}[1];

# Function calls
my $len = length("Hello");
my $substr = substr("Perl Programming", 0, 4);
my $max = max(10, 20, 30);
my $result = sqrt(abs(-16));

# Method calls
my $obj = MyClass->new();
$obj->method1();
$obj->method2($arg1, $arg2);
my $value = $obj->get_value();

# Package declaration
package MyPackage;

sub new {
    my $class = shift;
    return bless {}, $class;
}

1;  # Return true

=head1 NAME

Test - Comprehensive parser test

=cut
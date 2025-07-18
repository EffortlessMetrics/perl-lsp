#!/usr/bin/perl
use strict;
use warnings;

=pod
Comprehensive test of Pure Rust Perl parser
This showcases all implemented features
=cut

# Variables and declarations
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value', foo => 'bar');
our $global = "global";
local $package_var = "local";

# String interpolation
my $name = "World";
my $greeting = "Hello, $name!";
my $array_str = "Array: @array";
my $count = 10;
my $message = "Count is $count items";

# Operators
my $sum = 5 + 3;
my $diff = 10 - 2;
my $product = 4 * 3;
my $quotient = 20 / 4;
my $modulo = 17 % 5;
my $power = 2 ** 8;
my $concat = "Hello" . " " . "World";
my @range = (1..10);

# Comparison operators
my $eq = $a == $b;
my $ne = $a != $b;
my $lt = $a < $b;
my $gt = $a > $b;
my $le = $a <= $b;
my $ge = $a >= $b;
my $seq = $a eq $b;
my $sne = $a ne $b;

# Logical operators
my $and = $a && $b;
my $or = $a || $b;
my $not = !$a;
my $and_word = $a and $b;
my $or_word = $a or $b;
my $not_word = not $a;

# Control flow
if ($scalar > 40) {
    print "Greater than 40\n";
} elsif ($scalar == 40) {
    print "Equal to 40\n";
} else {
    print "Less than 40\n";
}

unless ($scalar < 0) {
    print "Non-negative\n";
}

# Loops
for (my $i = 0; $i < 10; $i++) {
    print "$i ";
}

foreach my $item (@array) {
    print "Item: $item\n";
}

while ($count > 0) {
    $count--;
    last if $count == 5;
}

until ($count == 10) {
    $count++;
    next if $count % 2 == 0;
    print "$count ";
}

# Statement modifiers
print "Hello\n" if $scalar > 0;
print "World\n" unless $scalar < 0;
$count++ while $count < 20;
$count-- until $count == 10;

# Subroutines
sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

sub greet :lvalue {
    my $name = shift;
    print "Hello, $name!\n";
}

# Anonymous subroutines
my $anon = sub {
    my $x = shift;
    return $x * 2;
};

my $result = $anon->(21);

# Array and hash access
my $first = $array[0];
my $last = $array[-1];
my $value = $hash{key};
$array[1] = 42;
$hash{new} = "value";

# Complex dereferencing
my $complex = $data->{users}[0]{name};
my $nested = $obj->{method}->($arg)->[0];

# Method calls
my $obj = Class->new();
$obj->method();
$obj->method($arg1, $arg2);

# Function calls
my $len = length($string);
my $substr = substr($string, 0, 5);
my $joined = join(", ", @array);

# Regular expressions
my $re1 = qr/pattern/;
my $re2 = qr/foo|bar/i;
my $re3 = qr{nested{brackets}};

# Package and use
package MyPackage;
use Data::Dumper;
use warnings qw(all);

# Return
return 1;
#!/usr/bin/perl
# Test anonymous subroutines
my $add = sub {
    my ($a, $b) = @_;
    return $a + $b;
};

my $result = $add->(5, 3);

# Anonymous sub as argument
my @sorted = sort { $a cmp $b } @array;

# Anonymous sub with prototype
my $proto_sub = sub ($$) {
    my ($x, $y) = @_;
    return $x * $y;
};
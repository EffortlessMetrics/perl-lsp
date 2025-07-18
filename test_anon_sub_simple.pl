my $add = sub { return $_[0] + $_[1]; };
my $mul = sub ($x, $y) { return $x * $y; };
print $add->(5, 3);
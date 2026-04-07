#!/usr/bin/env perl
# Valid C-style for loop test fixtures
# Tests for AC3: For-loop tuple validation

# Basic C-style for loop
for (my $i = 0; $i < 10; $i++) {
    print "$i\n";
}

# C-style with complex initialization
for (my ($x, $y) = (0, 0); $x < 10; $x++, $y += 2) {
    print "$x, $y\n";
}

# C-style with multiple statements
for (my $i = 0; $i < 10; $i++) {
    my $square = $i * $i;
    print "$i squared = $square\n";
}

# Empty initialization
for (; $condition; $i++) {
    print "loop\n";
}

# Empty condition (infinite loop)
for (my $i = 0; ; $i++) {
    last if $i >= 10;
}

# Empty update
for (my $i = 0; $i < 10; ) {
    print "$i\n";
    $i++;
}

# All parts empty (infinite loop)
for (;;) {
    last if $done;
}

# Complex condition
for (my $i = 0; $i < @array && $i < 100; $i++) {
    print $array[$i];
}

# Complex update
for (my $i = 0; $i < 10; $i = $i * 2 + 1) {
    print "$i\n";
}

#!/usr/bin/env perl
# Nested loop test fixtures
# Tests for AC3: For-loop tuple validation with nested structures

# C-style nested in C-style
for (my $i = 0; $i < 10; $i++) {
    for (my $j = 0; $j < 10; $j++) {
        print "($i, $j)\n";
    }
}

# Foreach nested in foreach
foreach my $row (@matrix) {
    foreach my $col (@$row) {
        print "$col ";
    }
    print "\n";
}

# C-style nested in foreach
foreach my $array (@arrays) {
    for (my $i = 0; $i < scalar(@$array); $i++) {
        print "$array->[$i]\n";
    }
}

# Foreach nested in C-style
for (my $i = 0; $i < @arrays; $i++) {
    foreach my $item (@{$arrays[$i]}) {
        print "$item\n";
    }
}

# Triple nesting
foreach my $x (@x_coords) {
    foreach my $y (@y_coords) {
        foreach my $z (@z_coords) {
            print "($x, $y, $z)\n";
        }
    }
}

# Labeled nested loops
OUTER: for (my $i = 0; $i < 10; $i++) {
    MIDDLE: for (my $j = 0; $j < 10; $j++) {
        INNER: for (my $k = 0; $k < 10; $k++) {
            next OUTER if $condition;
            last MIDDLE if $other_condition;
        }
    }
}

# Complex nested with both styles
foreach my $file (@files) {
    open my $fh, '<', $file or next;
    for (my $line_num = 1; my $line = <$fh>; $line_num++) {
        print "$file:$line_num: $line";
    }
    close $fh;
}

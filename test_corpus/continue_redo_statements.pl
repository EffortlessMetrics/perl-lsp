use strict;
use warnings;

# Basic loop filtering and flow control
my @items = (1, undef, 2, 3);
for my $item (@items) {
    next unless defined $item;
    print $item;
}

# Continue blocks for while/until/for
my $while_count = 0;
while ($while_count < 3) {
    $while_count++;
    redo if $while_count == 2;
} continue {
    my $seen = $while_count;
}

my $until_count = 0;
until ($until_count >= 3) {
    $until_count++;
    next if $until_count == 1;
} continue {
    my $after_until = $until_count * 2;
}

for my $n (1 .. 5) {
    last if $n == 4;
    print $n;
} continue {
    my $after_for = $n + 10;
}

# Labeled redo/next/last interactions
OUTER: for my $i (1 .. 3) {
    INNER: for my $j (1 .. 3) {
        next OUTER if $i == $j;
        redo INNER if $j == 1;
        last INNER if $j == 3;
        print "$i,$j\n";
    }
} continue {
    my $outer_after = $i * 2;
}

# Continue does not execute after last in same iteration
my $guard = 0;
for my $v (1 .. 3) {
    $guard++;
    last if $v == 2;
} continue {
    my $continue_guard = $guard;
}

# Nested continue blocks with explicit labels
PHASE: while ($guard < 5) {
    $guard++;
    STEP: for my $step (1 .. 2) {
        redo STEP if $step == 1;
        next PHASE if $guard == 4;
        print "$guard/$step\n";
    } continue {
        my $inner_continue = $step;
    }
} continue {
    my $phase_continue = $guard;
}

use strict;
use warnings;

my @items = (1, undef, 2, 3);

for my $item (@items) {
    next unless defined $item;
    print $item;
}

my $count = 0;
while ($count < 3) {
    $count++;
    redo if $count == 2;
}

OUTER: for my $i (1..3) {
    INNER: for my $j (1..3) {
        next OUTER if $i == $j;
        redo INNER if $j == 1;
        print "$i,$j\n";
    }
} continue {
    my $after = $i * 2;
}

my $value = 0;
until ($value > 3) {
    $value++;
    next if $value == 1;
}

for my $n (1..5) {
    last if $n == 4;
    print $n;
}

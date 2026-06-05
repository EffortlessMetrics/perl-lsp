use strict;
use warnings;

for my $item (1 .. 3) {
    next MISSING_NEXT if $item == 1;
    last MISSING_LAST if $item == 2;
    redo MISSING_REDO if $item == 3;
}

FOUND:
for my $item (1 .. 2) {
    next FOUND if $item == 1;
}

package Demo::Symbols;

use strict;
use warnings;

sub alpha {
    return 1;
}

sub beta {
    return alpha();
}

1;

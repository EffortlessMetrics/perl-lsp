package NormalStaticMissingSymbol;

use strict;
use warnings;

# Control fixture: no dynamic boundary at all.
# 'truly_undefined_sub' is statically missing — diagnostic MUST fire.
# This is a high-confidence normal missing symbol.
sub defined_sub {
    return 1;
}

my $result = defined_sub();

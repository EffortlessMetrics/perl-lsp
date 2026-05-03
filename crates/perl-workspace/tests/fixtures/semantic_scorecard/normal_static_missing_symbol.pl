package NormalStaticMissingSymbol;

use strict;
use warnings;

# Control fixture: no dynamic boundary at all.
# `truly_undefined_sub` is statically missing — diagnostic MUST fire.
# This proves dynamic-boundary suppression does not become a broad
# "shut up" switch for ordinary high-confidence missing-symbol errors.
sub defined_sub {
    return 1;
}

my $result = defined_sub();
truly_undefined_sub();

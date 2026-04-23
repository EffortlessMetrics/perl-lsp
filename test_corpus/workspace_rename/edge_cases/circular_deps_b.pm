package CircularB;
# Test: Circular module dependencies (file B)
# Input: Rename function_b to renamed_b

use strict;
use warnings;
use CircularA;

sub function_b {
    my ($data) = @_;
    return "B: $data";
}

sub call_a {
    return CircularA::function_a("from B");
}

1;

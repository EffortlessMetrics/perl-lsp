package CircularA;
# Test: Circular module dependencies (file A)
# Input: Rename function_a to renamed_a

use strict;
use warnings;
use CircularB;

sub function_a {
    my ($data) = @_;
    return "A: $data";
}

sub call_b {
    return CircularB::function_b("from A");
}

1;

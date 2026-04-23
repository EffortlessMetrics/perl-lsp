package Package;
# Test: Package scope conflict detection
# Input: Rename old_name to new_name (should fail - new_name exists)

use strict;
use warnings;

sub old_name {
    return "old implementation";
}

sub new_name {
    return "existing implementation";
}

package Other;

sub old_name {
    return "other package old";
}

1;

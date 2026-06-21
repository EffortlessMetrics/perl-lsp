package Snapshot::Parent;

use strict;
use warnings;

our @ISA = ('Snapshot::Grandparent');

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub method_a {
    return 1;
}

package Snapshot::Child;

use parent -norequire, 'Snapshot::Parent';

sub method_b {
    return 2;
}

1;

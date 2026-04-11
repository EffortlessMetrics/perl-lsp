package Module944;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub compute_944 {
    my ($self, $x, $y) = @_;
    return $x + $y + 944;
}

1;

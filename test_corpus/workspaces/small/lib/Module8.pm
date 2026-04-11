package Module8;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub helper_8 {
    my ($self, $x) = @_;
    return $x * 8;
}

1;

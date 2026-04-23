package Module23;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_23 {
    my ($self, $data) = @_;
    return "processed_23: $data";
}

sub transform_23 {
    my ($self, $value) = @_;
    return $value + 23;
}

1;

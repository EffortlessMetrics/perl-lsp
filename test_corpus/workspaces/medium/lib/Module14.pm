package Module14;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_14 {
    my ($self, $data) = @_;
    return "processed_14: $data";
}

sub transform_14 {
    my ($self, $value) = @_;
    return $value + 14;
}

1;

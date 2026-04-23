package Module13;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_13 {
    my ($self, $data) = @_;
    return "processed_13: $data";
}

sub transform_13 {
    my ($self, $value) = @_;
    return $value + 13;
}

1;

package Module32;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_32 {
    my ($self, $data) = @_;
    return "processed_32: $data";
}

sub transform_32 {
    my ($self, $value) = @_;
    return $value + 32;
}

1;

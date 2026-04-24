package Module11;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_11 {
    my ($self, $data) = @_;
    return "processed_11: $data";
}

sub transform_11 {
    my ($self, $value) = @_;
    return $value + 11;
}

1;

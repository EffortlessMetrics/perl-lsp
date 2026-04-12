package Module38;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_38 {
    my ($self, $data) = @_;
    return "processed_38: $data";
}

sub transform_38 {
    my ($self, $value) = @_;
    return $value + 38;
}

1;

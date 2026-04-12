package Module22;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_22 {
    my ($self, $data) = @_;
    return "processed_22: $data";
}

sub transform_22 {
    my ($self, $value) = @_;
    return $value + 22;
}

1;

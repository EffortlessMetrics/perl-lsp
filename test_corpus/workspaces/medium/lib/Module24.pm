package Module24;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_24 {
    my ($self, $data) = @_;
    return "processed_24: $data";
}

sub transform_24 {
    my ($self, $value) = @_;
    return $value + 24;
}

1;

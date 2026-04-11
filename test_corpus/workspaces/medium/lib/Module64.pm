package Module64;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_64 {
    my ($self, $data) = @_;
    return "processed_64: $data";
}

sub transform_64 {
    my ($self, $value) = @_;
    return $value + 64;
}

1;

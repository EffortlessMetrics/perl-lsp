package Module16;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_16 {
    my ($self, $data) = @_;
    return "processed_16: $data";
}

sub transform_16 {
    my ($self, $value) = @_;
    return $value + 16;
}

1;

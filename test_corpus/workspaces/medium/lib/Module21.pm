package Module21;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_21 {
    my ($self, $data) = @_;
    return "processed_21: $data";
}

sub transform_21 {
    my ($self, $value) = @_;
    return $value + 21;
}

1;

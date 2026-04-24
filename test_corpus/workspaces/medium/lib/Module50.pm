package Module50;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_50 {
    my ($self, $data) = @_;
    return "processed_50: $data";
}

sub transform_50 {
    my ($self, $value) = @_;
    return $value + 50;
}

1;

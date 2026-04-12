package Module90;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_90 {
    my ($self, $data) = @_;
    return "processed_90: $data";
}

sub transform_90 {
    my ($self, $value) = @_;
    return $value + 90;
}

1;

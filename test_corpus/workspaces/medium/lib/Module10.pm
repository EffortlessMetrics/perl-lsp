package Module10;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_10 {
    my ($self, $data) = @_;
    return "processed_10: $data";
}

sub transform_10 {
    my ($self, $value) = @_;
    return $value + 10;
}

1;

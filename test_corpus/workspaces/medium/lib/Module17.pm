package Module17;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_17 {
    my ($self, $data) = @_;
    return "processed_17: $data";
}

sub transform_17 {
    my ($self, $value) = @_;
    return $value + 17;
}

1;

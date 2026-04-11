package Module18;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_18 {
    my ($self, $data) = @_;
    return "processed_18: $data";
}

sub transform_18 {
    my ($self, $value) = @_;
    return $value + 18;
}

1;

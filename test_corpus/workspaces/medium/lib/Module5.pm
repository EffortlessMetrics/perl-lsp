package Module5;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_5 {
    my ($self, $data) = @_;
    return "processed_5: $data";
}

sub transform_5 {
    my ($self, $value) = @_;
    return $value + 5;
}

1;

package Module26;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_26 {
    my ($self, $data) = @_;
    return "processed_26: $data";
}

sub transform_26 {
    my ($self, $value) = @_;
    return $value + 26;
}

1;

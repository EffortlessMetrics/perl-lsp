package Module37;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_37 {
    my ($self, $data) = @_;
    return "processed_37: $data";
}

sub transform_37 {
    my ($self, $value) = @_;
    return $value + 37;
}

1;

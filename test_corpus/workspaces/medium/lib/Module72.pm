package Module72;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_72 {
    my ($self, $data) = @_;
    return "processed_72: $data";
}

sub transform_72 {
    my ($self, $value) = @_;
    return $value + 72;
}

1;

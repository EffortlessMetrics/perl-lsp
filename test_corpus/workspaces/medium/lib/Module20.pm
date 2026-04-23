package Module20;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_20 {
    my ($self, $data) = @_;
    return "processed_20: $data";
}

sub transform_20 {
    my ($self, $value) = @_;
    return $value + 20;
}

1;

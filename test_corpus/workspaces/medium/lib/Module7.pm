package Module7;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_7 {
    my ($self, $data) = @_;
    return "processed_7: $data";
}

sub transform_7 {
    my ($self, $value) = @_;
    return $value + 7;
}

1;

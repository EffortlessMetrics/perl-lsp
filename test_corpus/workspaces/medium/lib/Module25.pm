package Module25;
use strict;
use warnings;

our $VERSION = '1.00';

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub process_25 {
    my ($self, $data) = @_;
    return "processed_25: $data";
}

sub transform_25 {
    my ($self, $value) = @_;
    return $value + 25;
}

1;

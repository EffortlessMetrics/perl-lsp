package Snapshot::UseStrict;

use strict;
use warnings;
use Carp qw(croak);
use Scalar::Util qw(blessed);

our @EXPORT_OK = qw(do_thing);

sub do_thing {
    my ($arg) = @_;
    croak "need arg" unless defined $arg;
    return blessed($arg) // 'scalar';
}

1;

package My::App;

use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub run {
    my ($self) = @_;
    my $x = 1;
    my $y = $x + 1;
    print "Result: $y\n";
    return $y;
}

1;

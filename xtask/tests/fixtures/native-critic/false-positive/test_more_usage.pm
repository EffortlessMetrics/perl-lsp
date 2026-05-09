package Clean::TestMoreUsage;
use strict;
use warnings;
use Test::More;

=head1 NAME

Clean::TestMoreUsage - Test::More native critic fixture

=head1 DESCRIPTION

Keeps common Test::More assertions quiet under native critic.

=cut

sub add {
    my ($left, $right) = @_;
    return $left + $right;
}

sub run_tests {
    plan tests => 2;
    ok(add(1, 1) == 2, 'addition works');
    is(add(2, 3), 5, 'addition returns expected result');
    return 1;
}

1;

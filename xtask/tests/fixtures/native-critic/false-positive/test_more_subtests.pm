package Clean::TestMoreSubtests;
use strict;
use warnings;
use Test::More;

=head1 NAME

Clean::TestMoreSubtests - Test::More subtest native critic fixture

=head1 DESCRIPTION

Keeps common subtest and done_testing idioms quiet under native critic.

=cut

sub multiply {
    my ($left, $right) = @_;
    return $left * $right;
}

sub run_tests {
    subtest 'multiplication' => sub {
        is(multiply(2, 3), 6, 'multiplies integers');
        ok(multiply(1, 0) == 0, 'zero works');
    };
    done_testing();
    return 1;
}

1;

package Clean::ClosureAndEval;
use strict;
use warnings;

=head1 NAME

Clean::ClosureAndEval - closure and eval native critic fixture

=head1 DESCRIPTION

Keeps closure captures and guarded eval handling quiet under native critic.

=cut

sub make_counter {
    my ($start) = @_;
    my $count = $start;
    return sub {
        $count += 1;
        return $count;
    };
}

sub parse_value {
    my ($source) = @_;
    my $result = eval { 0 + $source };
    my $error = $@;
    if (defined $error && length $error) {
        return undef;
    }
    return $result;
}

1;

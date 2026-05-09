package Clean::LocalizedEval;
use strict;
use warnings;

=head1 NAME

Clean::LocalizedEval - localized eval native critic fixture

=head1 DESCRIPTION

Keeps localized block eval error handling quiet under native critic.

=cut

sub parse_number {
    my ($source) = @_;
    my $value;
    {
        local $@;
        my $ok = eval {
            $value = 0 + $source;
            1;
        };
        if (!$ok) {
            return undef;
        }
    }
    return $value;
}

1;

package Clean::RegexCaptureGuarded;
use strict;
use warnings;

=head1 NAME

Clean::RegexCaptureGuarded - guarded regex capture native critic fixture

=head1 DESCRIPTION

Keeps capture-variable reads quiet when guarded by a successful regex match.

=cut

sub first_word {
    my ($text) = @_;
    if ($text =~ /(\w+)/) {
        return $1;
    }
    return undef;
}

1;

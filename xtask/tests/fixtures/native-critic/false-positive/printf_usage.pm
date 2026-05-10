package Clean::PrintfUsage;
use strict;
use warnings;

=head1 NAME

Clean::PrintfUsage - printf native critic fixture

=head1 DESCRIPTION

Keeps correctly matched printf and sprintf calls quiet under native critic.

=cut

sub render {
    my ($name, $count) = @_;
    my $label = sprintf "%s:%d", $name, $count;
    printf "%s\n", $label;
    return $label;
}

1;

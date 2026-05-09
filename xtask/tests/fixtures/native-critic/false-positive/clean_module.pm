package Clean::Module;
use strict;
use warnings;

=head1 NAME

Clean::Module - clean native critic fixture

=head1 SYNOPSIS

    Clean::Module::sum(1, 2);

=head1 DESCRIPTION

Exercises common clean Perl idioms that should not produce native critic findings.

=cut

sub sum {
    my ($left, $right) = @_;
    my $total = $left + $right;
    return $total;
}

sub count_items {
    my (@items) = @_;
    my $count = 0;
    for my $item (@items) {
        $count += length $item;
    }
    return $count;
}

1;

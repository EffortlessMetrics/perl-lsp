package Clean::CheckedIO;
use strict;
use warnings;

=head1 NAME

Clean::CheckedIO - checked IO native critic fixture

=head1 DESCRIPTION

Keeps three-argument open, checked close, and explicit print-handle usage quiet.

=cut

sub write_lines {
    my ($path, @lines) = @_;
    my $fh = undef;
    open($fh, '>', $path) or die "cannot open $path: $!";
    for my $line (@lines) {
        print {$fh} $line, "\n";
    }
    close($fh) or die "cannot close $path: $!";
    return scalar @lines;
}

1;

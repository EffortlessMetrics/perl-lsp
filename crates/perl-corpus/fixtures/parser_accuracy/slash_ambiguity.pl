package Accuracy::SlashAmbiguity;

sub classify_slashes {
    my ($total, $count, $line, $maybe) = @_;
    my $ratio = $total / $count;
    my @parts = split /,/, $line;
    my $matched = $line =~ /^ok:/;
    my $fallback = $maybe // $ratio;
    return ($ratio, @parts, $matched, $fallback);
}

1;

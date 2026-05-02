package Accuracy::NegativeSymbolRegions;

# sub commented_out { return 1 }
my $text = "sub stringy { return 1 }";

=pod
sub podded { return 1 }
=cut

BEGIN { make_accessor("dynamic_name") }

sub real { return 1 }

1;

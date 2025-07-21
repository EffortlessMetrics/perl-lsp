package MyModule;
use strict;
use warnings;

# Reference operator test
my $scalar = "test";
my $ref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern features
my $perms = 0o755;
sub todo { ... }
my $π = 3.14159;
my $café = "coffee";

# Slash disambiguation
sub calculate {
    my ($x, $y) = @_;
    return $x / $y if $y != 0;
    return 0 if $x =~ /^0+$/;
}

# Heredoc
my $config = <<'EOF2';
path: /usr/local/bin
regex: /\w+/
EOF2

1;

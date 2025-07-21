#!/usr/bin/env perl
use strict;
use warnings;

# Test various Perl features
my $scalar = "Hello, World!";
my @array = (1, 2, 3, 4, 5);
my %hash = (a => 1, b => 2, c => 3);

# Reference operator test
my $ref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Modern octal
my $perms = 0755;
my $modern = 0o755;

# Unicode
my $π = 3.14159;
my $café = "coffee";

# Ellipsis
sub todo { ... }

# Heredoc
my $heredoc = <<'EOF';
This is a heredoc
with multiple lines
EOF

# Regex
if ($scalar =~ /Hello/) {
    print "Match!\n";
}

# Substitution
$scalar =~ s/World/Perl/;

# String interpolation
print "The value is: $scalar\n";
print "Array: @array\n";

# Control structures
for my $i (1..10) {
    next if $i % 2;
    print "$i is even\n" if $i > 5;
}

# Subroutine
sub greet {
    my $name = shift;
    return "Hello, $name!";
}

print greet("Perl"), "\n";
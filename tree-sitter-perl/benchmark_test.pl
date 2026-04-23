#!/usr/bin/env perl
use strict;
use warnings;

# Test reference operator (\)
my $scalar = "Hello";
my @array = (1..10);
my %hash = (a => 1, b => 2);
my $sref = \$scalar;
my $aref = \@array;
my $href = \%hash;

# Test modern octal (0o755) and traditional
my $modern_perms = 0o755;
my $old_perms = 0755;

# Test ellipsis operator (...)
sub not_implemented {
    ...  # Placeholder
}

# Unicode identifiers
my $π = 3.14159;
my $café = "coffee";
sub 日本語 { return "works" }

# Complex structures
for my $i (1..100) {
    if ($i % 15 == 0) {
        print "FizzBuzz\n";
    } elsif ($i % 3 == 0) {
        print "Fizz\n";
    } elsif ($i % 5 == 0) {
        print "Buzz\n";
    } else {
        print "$i\n";
    }
}

# Regex with substitutions
my $text = "The quick brown fox";
$text =~ s/quick/fast/g;
$text =~ s/brown/red/g;

# Method calls
package Foo;
sub new { bless {}, shift }
sub bar { "method called" }
my $obj = Foo->new();
$obj->bar();

# Heredoc
my $heredoc = <<'END_TEXT';
This is a heredoc
with multiple lines
END_TEXT

# Modern Perl features
given ($scalar) {
    when ("Hello") { print "Greeting\n" }
    default { print "Other\n" }
}

1;

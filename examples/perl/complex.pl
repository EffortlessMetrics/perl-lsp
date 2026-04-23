#!/usr/bin/perl
use strict;
use warnings;
use feature 'say';

# Edge cases that v3 parser handles perfectly

# 1. Regex with non-slash delimiters
my $text = "Hello World";
$text =~ m!Hello!;
$text =~ s{World}{Universe}g;
$text =~ tr[a-z][A-Z];

# 2. Indirect object syntax
my $fh;
open $fh, '<', 'file.txt' or die $!;
print $fh "Hello\n";
print STDERR "Error message\n";

# 3. Complex prototypes
sub mygrep(&@) {
    my $code = shift;
    grep { $code->() } @_;
}

sub with_underscore(_) {
    my $arg = shift // $_;
    return uc($arg);
}

# 4. Unicode identifiers
my $π = 3.14159;
my $Σ = 0;
$Σ += $_ for 1..10;

sub café { "coffee" }
my $♥ = "love";

# 5. Complex dereferencing
my $ref = \@array;
my @copy = @{$ref};
push @{$ref}, 6;

# 6. Format declarations
format STDOUT =
@<<<<<<< @|||||||| @>>>>>>>>
$name,   $title,   $value
.

# 7. Here documents
my $heredoc = <<'END';
This is a heredoc
with multiple lines
END

my $interpolated = <<"EOF";
Name: $name
Value: $π
EOF

# 8. Given/when (experimental)
use feature 'switch';
given ($value) {
    when (1) { say "one" }
    when (2) { say "two" }
    default { say "other" }
}

# 9. Subroutine signatures (5.20+)
use feature 'signatures';
no warnings 'experimental::signatures';

sub add($x, $y) {
    return $x + $y;
}

# 10. Postfix dereferencing (5.20+)
use feature 'postderef';
no warnings 'experimental::postderef';

my $aref = [1, 2, 3];
my @elements = $aref->@*;
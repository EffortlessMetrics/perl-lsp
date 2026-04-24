#!/usr/bin/env perl
# Valid pattern test fixture (should NOT trigger anti-pattern detectors)
# Tests for AC5: Anti-pattern detector exhaustive matching
# Ensures detectors don't produce false positives

use strict;
use warnings;

# Valid format (no heredoc)
format STDOUT =
@<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$field1, $field2
.

# Valid BEGIN block (no heredoc)
BEGIN {
    use lib './lib';
    $ENV{PATH} = '/usr/bin:/bin';
}

# Valid heredoc (not in BEGIN)
my $config = <<'END';
host = localhost
port = 8080
END

# Valid regex with static delimiters
my $str = "test string";
$str =~ s/foo/bar/;
$str =~ s#baz#qux#;
$str =~ s{old}{new};
$str =~ s[pattern][replacement];

# Valid quote operators
my $single = q/single quoted/;
my $double = qq{double quoted};
my $command = qx/ls -la/;
my @words = qw(word list here);

# Valid transliteration
$str =~ tr/a-z/A-Z/;
$str =~ y/0-9/a-j/;

# Valid regex compilation
my $regex1 = qr/pattern/i;
my $regex2 = qr#another#x;
my $regex3 = qr{complex pattern};

# Normal subroutine
sub process_data {
    my ($input) = @_;

    # Valid heredoc in normal subroutine
    my $sql = <<'SQL';
    SELECT * FROM users
    WHERE active = 1
    SQL

    return $input;
}

# Valid CHECK/INIT/END blocks (no heredoc)
CHECK {
    print "CHECK: validation passed\n";
}

INIT {
    print "INIT: initialization complete\n";
}

END {
    print "END: cleanup\n";
}

# Valid format without heredoc interaction
format REPORT =
Page @<<
$page_number

Name: @<<<<<<<<<<<<<<<<<<<<<<<<<
$name

Balance: @#####.##
$balance
.

# Valid conditional regex
my $pattern = $case_sensitive ? qr/Pattern/ : qr/pattern/i;

# Valid array of regexes (not dynamic delimiters)
my @patterns = (
    qr/foo/,
    qr/bar/,
    qr/baz/
);

print "All valid patterns - no anti-patterns detected\n";

1;

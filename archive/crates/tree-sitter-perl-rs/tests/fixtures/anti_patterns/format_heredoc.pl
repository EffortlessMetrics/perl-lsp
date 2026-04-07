#!/usr/bin/env perl
# Anti-pattern test fixture: format heredoc
# Tests for AC5: Anti-pattern detector exhaustive matching
# FormatHeredocDetector should identify these patterns

use strict;
use warnings;

# Format heredoc anti-pattern
# This pattern is problematic because format and heredoc interact unexpectedly

format STDOUT =
@<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
"This is a format with embedded heredoc"
.

my $heredoc = <<'END';
This heredoc is used within format context
END

# Another format heredoc pattern
format REPORT =
@>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
<<EOF
This is problematic
EOF
.

# Nested format and heredoc
sub problematic_function {
    format LOCAL =
    @<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
    my $data = <<'DATA';
    Nested heredoc in format
    DATA
    .
}

# Valid format (not anti-pattern)
format NORMAL =
@<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
$variable_name
.

# Valid heredoc (not anti-pattern)
my $normal_heredoc = <<'END';
This is a normal heredoc
without format interaction
END

print "Normal code continues\n";

1;

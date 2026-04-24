#!/usr/bin/env perl
# Anti-pattern test fixture: BEGIN-time heredoc
# Tests for AC5: Anti-pattern detector exhaustive matching
# BeginTimeHeredocDetector should identify these patterns

use strict;
use warnings;

# BEGIN-time heredoc anti-pattern
# This is problematic because heredocs in BEGIN blocks can cause parsing issues

BEGIN {
    my $config = <<'END';
    Configuration data loaded at BEGIN time
    This can cause issues with parser
    END

    print "BEGIN block with heredoc\n";
}

# Nested BEGIN with heredoc
BEGIN {
    BEGIN {
        my $nested = <<'NESTED';
        Nested BEGIN heredoc
        NESTED
    }
}

# BEGIN with multiple heredocs
BEGIN {
    my $first = <<'FIRST';
    First heredoc
    FIRST

    my $second = <<'SECOND';
    Second heredoc
    SECOND
}

# Valid BEGIN block (not anti-pattern)
BEGIN {
    my $normal = "normal string";
    print "Normal BEGIN block\n";
}

# Valid heredoc outside BEGIN (not anti-pattern)
my $normal_heredoc = <<'END';
This is a normal heredoc
outside BEGIN block
END

# CHECK block with heredoc (similar anti-pattern)
CHECK {
    my $check_data = <<'CHECK_DATA';
    Data in CHECK block
    CHECK_DATA
}

# INIT block with heredoc (similar anti-pattern)
INIT {
    my $init_data = <<'INIT_DATA';
    Data in INIT block
    INIT_DATA
}

# END block with heredoc (similar anti-pattern)
END {
    my $end_data = <<'END_DATA';
    Data in END block
    END_DATA
}

print "Normal code continues\n";

1;

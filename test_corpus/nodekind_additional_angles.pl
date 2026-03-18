#!/usr/bin/env perl
# Test: Additional clean NodeKind angles for rare constructs
# Impact: Improves corpus diversity for low-frequency NodeKinds in clean parses
# NodeKinds: Goto, DataSection, Readline, Typeglob, Format, PhaseBlock, Do, Transliteration

use strict;
use warnings;

BEGIN {
    our $BOOTSTRAPPED = 1;
}

sub dynamic_dispatch {
    my ($code_ref, @args) = @_;
    goto &$code_ref;
}

sub uppercase_joined {
    my (@parts) = @_;
    my $joined = do {
        my $value = join q{ }, @parts;
        $value =~ tr/a-z/A-Z/;
        $value;
    };
    return $joined;
}

format STDOUT =
@<<<<<<<<<<<<<<<<<<<<<<<< @>>>>>
$main::report_name,         $main::report_total
.

our ($report_name, $report_total) = ("summary", 12);
*REPORT = *STDOUT;

my $headline = dynamic_dispatch(\&uppercase_joined, qw(alpha beta));
print REPORT "$headline\n";

my $first_data_line = <DATA>;
print REPORT $first_data_line if defined $first_data_line;

write;

1;

__DATA__
payload line

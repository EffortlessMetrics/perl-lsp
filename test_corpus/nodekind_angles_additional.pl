#!/usr/bin/env perl
# Test: Additional low-frequency NodeKind angles
# Impact: Adds clean second-angle corpus coverage for goto, tie/untie,
#         typeglobs, glob(), and data sections in compact real syntax.
# NodeKinds: Goto, Tie, Untie, Typeglob, Glob, DataSection, Match

use strict;
use warnings;

sub dispatch_tail {
    goto &target_sub;
}

sub target_sub {
    return 'ok';
}

my $matched = 'alpha-42' =~ /^alpha/;

my $cache = {};
tie my %tied_cache, 'Tie::StdHash';
$cache->{status} = dispatch_tail() if $matched;
untie %tied_cache;

*LOG_HANDLE = *STDOUT;
print LOG_HANDLE "logged\n";

my @perl_files = glob('test_corpus/*.pl');
print scalar(@perl_files) . " files\n";

__DATA__
sample payload

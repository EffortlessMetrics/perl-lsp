#!/usr/bin/env perl
# Test: Additional low-frequency NodeKind angles
# Impact: Adds compact alternative clean corpus coverage for low-frequency
#         constructs using parser-proven syntax with minimal runtime noise.
# NodeKinds: Goto, Tie, Untie, Typeglob, Glob, DataSection, Match

use strict;
use warnings;

sub tail_dispatch {
    @_ = ('retargeted');
    goto &tail_target;
}

sub tail_target {
    return shift;
}

my $state = 'start';
my $matched = 0;

FLOW: {
    $state = 'middle';
    $matched = ($state =~ /^mid/);
    goto FLOW_END if $matched;
    $state = 'unreachable';
    FLOW_END:
    $state = tail_dispatch();
}

tie my %cache, 'Tie::IxHash';
$cache{status} = $state;
untie %cache;

tie my $scalar_value, 'Tie::Scalar';
untie $scalar_value;

*LOG_HANDLE = *STDOUT;
tie *LOG_HANDLE, 'Tie::StdHandle';
untie *LOG_HANDLE;

*alias_stdout = *STDOUT;
*helper = \&tail_target;
my $alias_ref = \*alias_stdout;
my $stdout_ref = \*STDOUT;

my @glob_fn = glob('test_corpus/*.pl');
my @glob_angle = <test_corpus/*.pl>;
my $single_log = glob '*.log';
my $glob_summary = scalar(@glob_fn) + scalar(@glob_angle) + ($single_log ? 1 : 0);

my $sink = [$matched, $alias_ref, $stdout_ref, $glob_summary, helper('helper-call')];

__END__
This trailing section is data, not Perl code.

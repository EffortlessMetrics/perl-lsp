#!/usr/bin/env perl
# Test: Additional low-frequency NodeKind angles
# Impact: Adds alternative clean corpus coverage for low-frequency constructs
#         using parser-proven syntax borrowed from existing dedicated fixtures.
# NodeKinds: Goto, Tie, Untie, Typeglob, Glob, DataSection, Match

use strict;
use warnings;

# --- Goto in two clean forms ---
sub tail_dispatch {
    @_ = ('retargeted');
    goto &tail_target;
}

sub tail_target {
    return shift;
}

my $state = 'start';
FLOW: {
    $state = 'middle';
    goto FLOW_END if $state =~ /^mid/;
    $state = 'unreachable';
    FLOW_END:
    $state = tail_dispatch();
}

# --- Tie / untie in multiple shapes ---
tie my %cache, 'Tie::IxHash';
$cache{status} = $state;
untie %cache;

tie my $scalar_value, 'Tie::Scalar';
untie $scalar_value;

tie *LOG_HANDLE, 'Tie::StdHandle';
untie *LOG_HANDLE;

# --- Typeglob and glob alternative angles ---
*alias_stdout = *STDOUT;
*helper = \&tail_target;
my $stdout_ref = \*STDOUT;

my @glob_fn = glob('test_corpus/*.pl');
my @glob_angle = <test_corpus/*.pl>;
my $single_log = glob '*.log';

print alias_stdout scalar(@glob_fn), "\n";
print helper('helper-call'), "\n" if $stdout_ref;
print scalar(@glob_angle) + ($single_log ? 1 : 0), "\n";

__END__
This trailing section is data, not Perl code.

#!/usr/bin/perl
use v5.20;
use strict;
use warnings;

my $path = '/etc/passwd';
open my $fh, '<', $path or die "Cannot open: $!";
while (<$fh>) {
    print $_;
}
close $fh;

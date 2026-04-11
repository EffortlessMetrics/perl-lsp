#!/usr/bin/perl
use v5.20;
use strict;
use warnings;

my $text = "line1\nline2\n\nline3\n";
{
    local $/ = "";
    open my $fh, '<', \$text or die $!;
    while (my $para = <$fh>) {
        print "Para: $para\n";
    }
}

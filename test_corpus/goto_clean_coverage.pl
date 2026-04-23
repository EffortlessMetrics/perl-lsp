#!/usr/bin/perl
use strict;
use warnings;

sub dispatch {
    return "ok";
}

my $state = "start";

START:
if ($state eq "start") {
    $state = "done";
    goto FINISH;
}

FINISH:
goto &dispatch;

#!/usr/bin/env perl
use strict;
use warnings;

BEGIN {  # Breakpoint on BEGIN block should work (line 5)
    # Breakpoint in BEGIN block should work (line 6)
    print "Initializing...\n";
    my $config = load_config();
}

sub load_config {  # Breakpoint should work (line 11)
    # Breakpoint in function called from BEGIN should work (line 12)
    return { debug => 1 };
}

my $main_var = 42;  # Breakpoint should work (line 16)

sub main_logic {  # Breakpoint should work (line 18)
    # Breakpoint in regular function should work (line 19)
    return $main_var * 2;
}

END {  # Breakpoint on END block should work (line 23)
    # Breakpoint in END block should work (line 24)
    print "Cleanup...\n";
}

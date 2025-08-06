#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Module;

# Main application entry point
sub main {
    my $module = Module->new();
    my $result = $module->process("test");
    
    if ($result) {
        print "Success: $result\n";
    } else {
        warn "Failed to process\n";
    }
    
    return 0;
}

# Run the main function
exit main() unless caller;
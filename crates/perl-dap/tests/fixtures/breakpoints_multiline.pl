#!/usr/bin/env perl
use strict;
use warnings;

sub test_multiline {  # Breakpoint should work (line 5)
    my $result = some_function(  # Breakpoint on multiline statement start should work (line 6)
        arg1 => 'value1',      # Breakpoint on continuation lines? (line 7)
        arg2 => 'value2',      # Should validate against AST (line 8)
        arg3 => 'value3'       # Line 9
    );  # Breakpoint on closing should work (line 10)

    my $hash = {  # Breakpoint on hash start should work (line 12)
        key1 => 'val1',  # Line 13
        key2 => 'val2',  # Line 14
    };  # Line 15

    return $result;  # Breakpoint should work (line 17)
}

sub some_function {  # Breakpoint should work (line 20)
    my %args = @_;
    return join(',', values %args);
}

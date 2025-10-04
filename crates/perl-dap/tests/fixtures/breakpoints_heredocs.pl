#!/usr/bin/env perl
use strict;
use warnings;

sub test_heredocs {  # Breakpoint should work here (line 5)
    my $heredoc = <<'END_TEXT';  # Breakpoint on heredoc start should work (line 6)
This is a heredoc.
Breakpoints inside heredocs should not be allowed.
Only the statement initiating the heredoc should have a breakpoint.
END_TEXT

    my $result = length($heredoc);  # Breakpoint should work here (line 12)
    return $result;
}

my $another_heredoc = <<"END_INTERPOLATED";  # Breakpoint on heredoc start should work (line 16)
Interpolated heredoc
with $variables
END_INTERPOLATED

print "Done\n";  # Breakpoint should work here (line 21)

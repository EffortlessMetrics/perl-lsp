#!/usr/bin/env perl

# Test different heredoc types
my $basic = <<EOF;
Basic heredoc
EOF

my $quoted = <<'QUOTED';
Non-interpolated heredoc
QUOTED

my $indented = <<~INDENT;
    Indented heredoc
    with spaces
    INDENT

print "Done\n";
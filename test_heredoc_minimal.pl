#!/usr/bin/perl

# Test 1: Heredoc in eval string
eval 'my $x = <<EOF;
test
EOF
print $x;';

# Test 2: Heredoc in s///e  
my $y = "X";
$y =~ s/X/<<HD/e;
replaced
HD
#!/usr/bin/perl

# Issue 1: Heredoc in s///e 
# Current parser sees this as separate tokens
my $x = "X";
$x =~ s/X/<<EOF/e;
heredoc content
EOF

# Issue 2: What Perl actually does
# From deparse, we know this becomes:
# $x =~ s/X/"heredoc content\n";/e;

# Issue 3: More complex s///e with heredoc
my $y = "Y";
$y =~ s/Y/do { <<HD }/e;
in do block
HD

# Issue 4: Heredoc in eval string
# This works differently - the heredoc is part of the string
eval 'my $z = <<END;
eval heredoc
END
';

# Issue 5: Multiple heredocs in expression
my $multi = (<<A) . (<<B);
First
A
Second  
B
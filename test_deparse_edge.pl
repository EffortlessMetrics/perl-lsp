#!/usr/bin/perl

# Edge cases for heredoc parsing

# 1. Heredoc in s///e with expression
my $text1 = "X";
$text1 =~ s/X/<<EOF . "suffix"/e;
Heredoc
EOF

# 2. Nested eval with heredoc
eval {
    eval 'my $x = <<INNER;
Nested
INNER
print $x;';
};

# 3. Heredoc in code block within s///e
my $text2 = "Y";
$text2 =~ s/Y/do { my $h = <<HD; chomp $h; $h }/e;
In block
HD

# 4. Multiple heredocs in complex expression
my $complex = (<<A) . (<<B) . (<<C);
Part A
A
Part B
B
Part C
C

# 5. Heredoc in map
my @list = map { <<MAP } (1..2);
Item $_
MAP

# 6. Heredoc with same delimiter
my $d1 = <<EOF;
First EOF
EOF
my $d2 = <<EOF;
Second EOF
EOF